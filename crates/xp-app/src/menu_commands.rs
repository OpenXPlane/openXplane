//! Native routing for toggle_main_menu/settings in 0x14150bfe0.
//! These requests preserve helper-call order; window bodies/rendering are not ported here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    ToggleMainMenu,
    ToggleSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    QueryMainMenu,
    QueryState,
    CloseMainMenu,
    EnsureMainMenu,
    Refresh,
    ToggleSettingsWindow,
}

/// Query values must come from the native-equivalent window and engine-state services.
/// QueryState is only requested when the first query returns true. This routing does
/// not establish the meaning of the engine-state predicate or execute window helpers.
pub fn requests(command: Command, main_menu_query: bool, state_query: bool) -> Vec<Request> {
    use Request::*;
    match command {
        Command::ToggleSettings => vec![ToggleSettingsWindow],
        Command::ToggleMainMenu => {
            let mut out = vec![QueryMainMenu];
            if main_menu_query {
                out.push(QueryState);
                if state_query {
                    out.push(CloseMainMenu);
                    return out;
                }
            }
            out.extend([EnsureMainMenu, Refresh]);
            out
        }
    }
}
