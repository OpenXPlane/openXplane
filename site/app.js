// openXplane site: hash-routed single page, documents are Markdown files under docs/.
// Set REPO to the repository URL (currently the organization page); the link is hidden while it is empty.
const REPO = "https://github.com/OpenXPlane/openXplane";
// Discord invite link (https://discord.gg/...). Empty hides every Discord link.
const DISCORD = "";

const DOCS = [
  ["README", "Overview and commands"],
  ["ARCHITECTURE", "Architecture and roadmap"],
  ["PLAN", "Plan for a full implementation"],
  ["COMPATIBILITY", "Compatibility estimate"],
  ["research/BASELINE", "Reference build"],
  ["research/VERIFICATION", "Verification against the original"],
  ["research/ACF_LOADING", "ACF loading"],
  ["research/AIRFOILS", "AFL airfoils"],
  ["research/AERO_LOOKUP", "Aerodynamic lookup"],
  ["research/STALL_STATE", "Stall state"],
  ["research/BUFFET", "Stall buffet"],
  ["research/AFL_REGIMES", "AFL regimes"],
  ["research/PROFILE_PIPELINE", "Profile evaluator"],
  ["research/WING_ELEMENT", "Wing element (partial)"],
  ["research/DATA_OUTPUT", "Data-output lines"],
  ["research/FLIGHT_MODEL", "Approximate flight model"],
  ["research/RUNTIME_TIME", "Runtime clock"],
  ["research/ACF_SCHEMA", "ACF loader schema"],
  ["research/DATAREFS", "Datarefs"],
  ["research/COMMANDS", "Commands"],
  ["research/KEYMAP", "Default keyboard map"],
  ["research/VIEWER", "Cessna viewer"],
  ["KEYBOARD", "Keyboard"],
  ["DISCORD", "Discord"],
  ["research/APT_DAT", "apt.dat airports"],
  ["research/INSTALL_LAYOUT", "Installation layout"],
  ["research/WORLD_GEOMETRY", "Airport geometry"],
];

// Compatibility estimate and roadmap come from docs/compat.json (generated numbers in docs/COMPATIBILITY.md).
let compat = null;
async function loadCompat() {
  if (compat) return compat;
  try {
    const res = await fetch("docs/compat.json");
    if (!res.ok) throw new Error(res.status);
    compat = await res.json();
  } catch {
    compat = null;
  }
  return compat;
}
function totals(c) {
  const sum = (k) => c.subsystems.reduce((a, s) => a + s.weight * s[k], 0);
  return { implemented: sum("implemented"), verified: sum("verified") };
}

const GALLERY = {
  anims: [
    ["assets/anim/takeoff.gif", "Takeoff from Seattle-Tacoma", "The Cessna 172 accelerates down runway 16L and climbs out; chase camera, approximate flight model."],
    ["assets/anim/orbit.gif", "Orbiting the Cessna 172 SP", "The OBJ8 exterior with textures, lighting and glass, drawn by wgpu."],
  ],
  airports: [
    ["assets/gallery/airport-KSEA.jpg", "Seattle-Tacoma (KSEA)", "Runways, taxiways and aprons with holes, from apt.dat."],
    ["assets/gallery/airport-KSFO.jpg", "San Francisco (KSFO)", "Four runways in apt.dat."],
    ["assets/gallery/airport-KJFK.jpg", "New York JFK (KJFK)", "Runways with concrete surface codes."],
    ["assets/gallery/airport-EGLL.jpg", "London Heathrow (EGLL)", "Two parallel runways."],
    ["assets/gallery/airport-RJTT.jpg", "Tokyo Haneda (RJTT)", "Four runways, two of them crossing."],
    ["assets/gallery/airport-LSZH.jpg", "Zurich (LSZH)", "Three runways in apt.dat."],
    ["assets/gallery/airport-YSSY.jpg", "Sydney (YSSY)", "Three runways in apt.dat."],
  ],
  aircraft: [
    ["assets/gallery/exterior-standard.jpg", "Cessna 172 SP", "The standard variant: fuselage, gear, propeller, wings, seats and glass."],
    ["assets/gallery/exterior-seaplane.jpg", "Cessna 172 SP seaplane", "The floats variant loads and flies in the model as well."],
  ],
  flight: [
    ["assets/gallery/ui-frame-rate.jpg", "Frame-rate line", "The reference build's data-output frame-rate line (labels from its code; layout, colour and the sample values are from a photograph of it)."],
    ["assets/gallery/flight-000.00s.jpg", "On the runway", "Standing on the gear at the start of runway 16L."],
    ["assets/gallery/flight-017.00s.jpg", "Takeoff roll", "About 17 seconds in, the nose rotating."],
    ["assets/gallery/flight-024.00s.jpg", "Liftoff", "The wheels leave the runway."],
    ["assets/gallery/flight-040.00s.jpg", "Climb-out", "Climbing over the airport at about 90 knots."],
  ],
};
const lightboxItems = [];

function openLightbox(index) {
  const lb = $("#lightbox");
  const [src, title, text] = lightboxItems[index];
  lb.dataset.index = index;
  $("#lb-img").src = src;
  $("#lb-img").alt = title;
  $("#lb-caption").textContent = `${title}: ${text}`;
  lb.hidden = false;
}
function stepLightbox(delta) {
  const n = lightboxItems.length;
  openLightbox((Number($("#lightbox").dataset.index) + delta + n) % n);
}

function renderGallery() {
  view.replaceChildren(clone("gallery"));
  lightboxItems.length = 0;
  for (const [id, key] of [["gallery-anims", "anims"], ["gallery-airports", "airports"], ["gallery-aircraft", "aircraft"], ["gallery-flight", "flight"]]) {
    const box = view.querySelector("#" + id);
    for (const item of GALLERY[key]) {
      const [src, title, text] = item;
      const index = key === "anims" ? -1 : lightboxItems.push(item) - 1;
      const fig = document.createElement("figure");
      const img = document.createElement("img");
      img.src = src; img.alt = title; img.loading = "lazy";
      if (index >= 0) img.addEventListener("click", () => openLightbox(index));
      const cap = document.createElement("figcaption");
      cap.innerHTML = "<b></b><br>";
      cap.querySelector("b").textContent = title;
      cap.append(text);
      fig.append(img, cap);
      box.append(fig);
    }
  }
}

const $ = (s) => document.querySelector(s);
const view = $("#view");

function clone(id) { return document.getElementById(id).content.cloneNode(true); }

function renderHome() {
  const frag = clone("home");
  if (REPO) { const h = frag.querySelector("#hero-repo"); h.href = REPO; h.hidden = false; }
  if (DISCORD) { const h = frag.querySelector("#hero-discord"); h.href = DISCORD; h.hidden = false; }
  view.replaceChildren(frag);
  loadCompat().then((c) => {
    const chip = view.querySelector("#compat-chip");
    if (!c || !chip) return;
    const t = totals(c);
    chip.querySelector("span:last-child").textContent =
      `≈${t.implemented.toFixed(0)}% implemented · ≈${Math.round(t.verified)}% verified identical`;
    chip.hidden = false;
  });
}

async function renderStatus() {
  const frag = clone("status");
  view.replaceChildren(frag);
  const c = await loadCompat();
  if (!c) { view.querySelector("#status-summary").textContent = "Could not load the status data."; return; }
  const t = totals(c);
  view.querySelector("#status-summary").innerHTML =
    `<b>About ${t.implemented.toFixed(0)}% implemented, about ${Math.round(t.verified)}% verified identical to the original.</b> ` +
    "<i>Implemented</i> counts work that exists in any form, including approximations; <i>verified identical</i> counts only what " +
    "has been confirmed against the original's own code. This is a self-assessed rubric, not a measurement.";
  view.querySelector("#status-method").textContent = c.method;
  const rows = view.querySelector("#status-rows");
  for (const s of c.subsystems) {
    const tr = document.createElement("tr");
    tr.innerHTML = '<td></td><td class="num"></td><td></td><td></td>';
    tr.children[0].textContent = s.name;
    tr.children[1].textContent = s.weight;
    const bar = (v, cls) => `<div class="bar" title="${(v * 100).toFixed(0)}%"><span class="${cls}" style="width:${v * 100}%"></span></div><small>${(v * 100).toFixed(0)}%</small>`;
    tr.children[2].innerHTML = bar(s.implemented, "impl");
    tr.children[3].innerHTML = bar(s.verified, "ver");
    const note = document.createElement("div");
    note.className = "note"; note.textContent = s.note;
    tr.children[0].append(note);
    rows.append(tr);
  }
  const ol = view.querySelector("#milestones");
  for (const r of c.roadmap) {
    const li = document.createElement("li");
    li.innerHTML = '<b></b> <span class="badge"></span><div class="note"></div>';
    li.querySelector("b").textContent = r.title;
    const b = li.querySelector(".badge");
    b.textContent = r.state;
    b.className = "badge " + ({ "mostly done": "done", "in progress": "part", started: "part" }[r.state] || "todo");
    li.querySelector(".note").textContent = r.detail;
    ol.append(li);
  }
  const ul = view.querySelector("#next-list");
  for (const n of c.next) { const li = document.createElement("li"); li.textContent = n; ul.append(li); }
}

async function renderDoc(name) {
  const known = DOCS.find(([n]) => n === name);
  if (!known) return notFound();
  view.textContent = "Loading…";
  try {
    const res = await fetch(`docs/${name}.md`);
    if (!res.ok) throw new Error(res.status);
    const html = marked.parse(await res.text());
    const box = document.createElement("article");
    box.innerHTML = html;
    // document links to sibling files become site routes
    for (const a of box.querySelectorAll("a[href]")) {
      const m = a.getAttribute("href").match(/^(?:\.\/|\.\.\/)?(?:docs\/|research\/)?([A-Za-z0-9_]+)\.md(#.*)?$/);
      if (!m) continue;
      const target = DOCS.find(([n]) => n === m[1] || n === `research/${m[1]}`);
      if (target) a.setAttribute("href", `#/doc/${target[0]}`);
    }
    // README images are repo-relative (site/assets/...); the site serves them from assets/
    for (const img of box.querySelectorAll("img[src]")) {
      img.setAttribute("src", img.getAttribute("src").replace(/^(\.\/)?site\//, ""));
    }
    view.replaceChildren(box);
  } catch {
    view.textContent = "Could not load the document.";
  }
}

function notFound() {
  view.innerHTML = "<h1>Page not found</h1><p><a href=\"#/\">Back to the overview</a></p>";
}

function route() {
  const path = location.hash.replace(/^#/, "") || "/";
  document.body.classList.remove("drawer-open");
  document.querySelectorAll(".drawer a").forEach((a) =>
    a.classList.toggle("active", a.dataset.route === path));
  window.scrollTo(0, 0);
  if (path === "/") renderHome();
  else if (path === "/gallery") renderGallery();
  else if (path === "/status") renderStatus();
  else if (path === "/download") {
    view.replaceChildren(clone("download"));
    const n = view.querySelector("#nightly-link");
    if (n && REPO) n.href = `${REPO}/releases/tag/nightly`;
  }
  else if (path.startsWith("/doc/")) renderDoc(path.slice(5));
  else notFound();
}

const links = $("#doc-links");
for (const [name, title] of DOCS) {
  const a = document.createElement("a");
  a.href = `#/doc/${name}`; a.dataset.route = `/doc/${name}`; a.textContent = title;
  links.append(a);
}
if (DISCORD) {
  const dl = $("#discord-link"); dl.href = DISCORD; dl.hidden = false;
  const pl = $("#project-links");
  const a = document.createElement("a");
  a.href = DISCORD; a.target = "_blank"; a.rel = "noopener";
  a.innerHTML = '<span class="material-icons">forum</span>';
  a.append("Discord");
  pl.append(a);
  const hd = $("#hero-discord"); if (hd) hd.href = DISCORD;
}
if (REPO) {
  const r = $("#repo-link"); r.href = REPO; r.hidden = false;
  $("#license-link").href = `${REPO}/blob/main/LICENSE`;
  const pl = $("#project-links");
  for (const [icon, title, href] of [
    ["code", "Source code", REPO],
    ["new_releases", "Releases", `${REPO}/releases`],
    ["bug_report", "Issues", `${REPO}/issues`],
  ]) {
    const a = document.createElement("a");
    a.href = href; a.target = "_blank"; a.rel = "noopener";
    a.innerHTML = `<span class="material-icons">${icon}</span>`;
    a.append(title);
    pl.append(a);
  }
}

$("#lb-close").addEventListener("click", () => { $("#lightbox").hidden = true; });
$("#lb-prev").addEventListener("click", () => stepLightbox(-1));
$("#lb-next").addEventListener("click", () => stepLightbox(1));
$("#lightbox").addEventListener("click", (e) => { if (e.target.id === "lightbox") $("#lightbox").hidden = true; });
addEventListener("keydown", (e) => {
  if ($("#lightbox").hidden) return;
  if (e.key === "Escape") $("#lightbox").hidden = true;
  else if (e.key === "ArrowLeft") stepLightbox(-1);
  else if (e.key === "ArrowRight") stepLightbox(1);
});
$("#menu-btn").addEventListener("click", () => document.body.classList.toggle("drawer-open"));
$("#scrim").addEventListener("click", () => document.body.classList.remove("drawer-open"));
$("#theme-btn").addEventListener("click", () => {
  const dark = matchMedia("(prefers-color-scheme: dark)").matches;
  const cur = document.documentElement.dataset.theme || (dark ? "dark" : "light");
  const next = cur === "dark" ? "light" : "dark";
  document.documentElement.dataset.theme = next;
  try { localStorage.setItem("theme", next); } catch {}
});
addEventListener("hashchange", route);
route();
