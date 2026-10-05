//! Discord Rich Presence over the local Discord IPC socket, standard library only.
//!
//! Wire format (Discord's documented RPC transport): every frame is a little-endian u32 opcode, a
//! little-endian u32 payload length and a JSON payload. Opcode 0 is the handshake
//! `{"v":1,"client_id":"..."}`, 1 is a command/event frame, 2 is close. The server answers the
//! handshake with a `READY` event; `SET_ACTIVITY` sets or (with a null activity) clears the status.
//! It needs a Discord application id, read from `OPENXPLANE_DISCORD_APP_ID`; without one presence
//! is simply off. Not verified against a running Discord client in this repository: the frame
//! encoding and the handshake/activity exchange are tested against a local mock socket.
use std::io::{Read, Write};

pub const APP_ID_VAR: &str = "OPENXPLANE_DISCORD_APP_ID";
const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const MAX_FRAME: usize = 1 << 20;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Activity {
    pub details: String,
    pub state: String,
    /// Seconds since the Unix epoch; Discord shows "elapsed" from it.
    pub start_timestamp: Option<u64>,
}

pub fn encode_frame(opcode: u32, payload: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload.as_bytes());
    out
}

pub fn read_frame<R: Read>(reader: &mut R) -> Result<(u32, String), String> {
    let mut head = [0u8; 8];
    reader
        .read_exact(&mut head)
        .map_err(|e| format!("discord: {e}"))?;
    let opcode = u32::from_le_bytes(head[..4].try_into().unwrap());
    let len = u32::from_le_bytes(head[4..].try_into().unwrap()) as usize;
    if len > MAX_FRAME {
        return Err(format!("discord: frame of {len} bytes is too large"));
    }
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .map_err(|e| format!("discord: {e}"))?;
    Ok((
        opcode,
        String::from_utf8(body).map_err(|_| "discord: payload is not UTF-8".to_string())?,
    ))
}

pub fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn handshake_payload(client_id: &str) -> Result<String, String> {
    if client_id.is_empty() || !client_id.chars().all(|c| c.is_ascii_digit()) {
        return Err("discord: the application id must be a number".into());
    }
    Ok(format!(
        "{{\"v\":1,\"client_id\":{}}}",
        json_escape(client_id)
    ))
}

/// Discord limits `details` and `state` to 128 characters and wants at least two.
fn clip(text: &str) -> String {
    let mut t: String = text.chars().take(128).collect();
    while t.chars().count() < 2 {
        t.push(' ');
    }
    t
}

pub fn activity_payload(pid: u32, nonce: u64, activity: Option<&Activity>) -> String {
    let body = match activity {
        None => "null".to_string(),
        Some(a) => {
            let mut s = format!(
                "{{\"details\":{},\"state\":{}",
                json_escape(&clip(&a.details)),
                json_escape(&clip(&a.state))
            );
            if let Some(t) = a.start_timestamp {
                s.push_str(&format!(",\"timestamps\":{{\"start\":{t}}}"));
            }
            s.push('}');
            s
        }
    };
    format!(
        "{{\"cmd\":\"SET_ACTIVITY\",\"args\":{{\"pid\":{pid},\"activity\":{body}}},\"nonce\":\"{nonce}\"}}"
    )
}

pub trait Transport: Read + Write {}
impl<T: Read + Write> Transport for T {}

pub struct Presence {
    stream: Box<dyn Transport>,
    pid: u32,
    nonce: u64,
}

impl Presence {
    /// Handshakes over an already open transport and waits for `READY`.
    pub fn handshake(mut stream: Box<dyn Transport>, client_id: &str) -> Result<Self, String> {
        let payload = handshake_payload(client_id)?;
        stream
            .write_all(&encode_frame(OP_HANDSHAKE, &payload))
            .map_err(|e| format!("discord: {e}"))?;
        let (op, body) = read_frame(&mut stream)?;
        if op == OP_CLOSE || !body.contains("\"READY\"") {
            return Err(format!("discord: handshake refused: {body}"));
        }
        Ok(Self {
            stream,
            pid: std::process::id(),
            nonce: 0,
        })
    }

    /// Connects to the local Discord client using the application id in `OPENXPLANE_DISCORD_APP_ID`.
    /// `Ok(None)` when no id is configured; an error when one is set but Discord cannot be reached.
    pub fn connect_from_env() -> Result<Option<Self>, String> {
        let Ok(id) = std::env::var(APP_ID_VAR) else {
            return Ok(None);
        };
        Self::connect(id.trim()).map(Some)
    }

    pub fn connect(client_id: &str) -> Result<Self, String> {
        Self::handshake(open_ipc()?, client_id)
    }

    pub fn set_activity(&mut self, activity: &Activity) -> Result<(), String> {
        self.send(Some(activity))
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.send(None)
    }

    fn send(&mut self, activity: Option<&Activity>) -> Result<(), String> {
        self.nonce += 1;
        let payload = activity_payload(self.pid, self.nonce, activity);
        self.stream
            .write_all(&encode_frame(OP_FRAME, &payload))
            .map_err(|e| format!("discord: {e}"))?;
        let (_, reply) = read_frame(&mut self.stream)?;
        if reply.contains("\"evt\":\"ERROR\"") {
            return Err(format!("discord: {reply}"));
        }
        Ok(())
    }
}

#[cfg(unix)]
fn open_ipc() -> Result<Box<dyn Transport>, String> {
    use std::os::unix::net::UnixStream;
    use std::time::Duration;
    let mut dirs: Vec<String> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(|d| d.trim_end_matches('/').to_string())
        .collect();
    dirs.push("/tmp".into());
    for dir in dirs {
        for sub in ["", "/app/com.discordapp.Discord", "/snap.discord"] {
            for n in 0..10 {
                let path = format!("{dir}{sub}/discord-ipc-{n}");
                if let Ok(stream) = UnixStream::connect(&path) {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                    return Ok(Box::new(stream));
                }
            }
        }
    }
    Err("discord: no running Discord client found".into())
}

#[cfg(windows)]
fn open_ipc() -> Result<Box<dyn Transport>, String> {
    for n in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{n}");
        if let Ok(file) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        {
            return Ok(Box::new(file));
        }
    }
    Err("discord: no running Discord client found".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn frames_round_trip_little_endian() {
        let frame = encode_frame(1, "{\"a\":1}");
        assert_eq!(&frame[..8], &[1, 0, 0, 0, 7, 0, 0, 0]);
        let (op, body) = read_frame(&mut Cursor::new(frame)).unwrap();
        assert_eq!((op, body.as_str()), (1, "{\"a\":1}"));
        assert!(read_frame(&mut Cursor::new(vec![1, 0, 0, 0, 5, 0])).is_err());
        let huge = [vec![1, 0, 0, 0], u32::MAX.to_le_bytes().to_vec()].concat();
        assert!(
            read_frame(&mut Cursor::new(huge))
                .unwrap_err()
                .contains("too large")
        );
    }

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_escape("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
    }

    #[test]
    fn handshake_requires_a_numeric_application_id() {
        assert_eq!(
            handshake_payload("1234").unwrap(),
            "{\"v\":1,\"client_id\":\"1234\"}"
        );
        assert!(handshake_payload("").is_err());
        assert!(handshake_payload("12\"34").is_err());
    }

    #[test]
    fn activity_payload_has_timestamps_and_clips_text() {
        let a = Activity {
            details: "Viewing".into(),
            state: "x".repeat(300),
            start_timestamp: Some(1700000000),
        };
        let p = activity_payload(42, 7, Some(&a));
        assert!(p.contains("\"pid\":42") && p.contains("\"nonce\":\"7\""));
        assert!(p.contains("\"timestamps\":{\"start\":1700000000}"));
        assert!(p.contains(&format!("\"state\":\"{}\"", "x".repeat(128))));
        assert!(activity_payload(1, 1, None).contains("\"activity\":null"));
        assert!(
            activity_payload(
                1,
                1,
                Some(&Activity {
                    details: "a".into(),
                    ..Activity::default()
                })
            )
            .contains("\"details\":\"a \"")
        );
    }

    #[cfg(unix)]
    #[test]
    fn handshake_and_activity_against_a_mock_discord() {
        use std::os::unix::net::UnixListener;
        let path =
            std::env::temp_dir().join(format!("openxplane-discord-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let (op, hello) = read_frame(&mut s).unwrap();
            assert_eq!(op, 0);
            assert_eq!(hello, "{\"v\":1,\"client_id\":\"987\"}");
            s.write_all(&encode_frame(1, "{\"cmd\":\"DISPATCH\",\"evt\":\"READY\"}"))
                .unwrap();
            let (op, set) = read_frame(&mut s).unwrap();
            assert_eq!(op, 1);
            assert!(set.contains("\"cmd\":\"SET_ACTIVITY\"") && set.contains("Cessna 172 SP"));
            s.write_all(&encode_frame(1, "{\"cmd\":\"SET_ACTIVITY\",\"evt\":null}"))
                .unwrap();
            let (_, clear) = read_frame(&mut s).unwrap();
            assert!(clear.contains("\"activity\":null"));
            s.write_all(&encode_frame(
                1,
                "{\"evt\":\"ERROR\",\"data\":{\"code\":4000}}",
            ))
            .unwrap();
        });
        let stream = std::os::unix::net::UnixStream::connect(&path).unwrap();
        let mut p = Presence::handshake(Box::new(stream), "987").unwrap();
        let a = Activity {
            details: "Viewing aircraft".into(),
            state: "Cessna 172 SP".into(),
            start_timestamp: None,
        };
        p.set_activity(&a).unwrap();
        assert!(p.clear().unwrap_err().contains("ERROR"));
        server.join().unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn refused_handshake_is_an_error() {
        struct Fake(Cursor<Vec<u8>>);
        impl Read for Fake {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                self.0.read(b)
            }
        }
        impl Write for Fake {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let reply = encode_frame(2, "{\"code\":4000,\"message\":\"Invalid Client ID\"}");
        let err = Presence::handshake(Box::new(Fake(Cursor::new(reply))), "1")
            .err()
            .unwrap();
        assert!(err.contains("refused"));
    }
}
