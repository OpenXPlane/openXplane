//! The `openxplane` facade over the workspace crates: one place to name everything the application and the
//! tests use.
pub use xp_acf::{Aircraft, Property, aircraft, reference_candidates, wing};
pub use xp_airfoil::{
    aero, airflow, airfoil, atmosphere, attitude, binding, blend, body, buffet, callees, controls,
    element_force, engine, engine_env, engine_small, flight_init, flight_state, flight_step,
    forces, fuel, gear, input, limits, mass, matrix, piston, profile, prop, regimes, runtime,
    scalar, shadow, stall, strut, terrain, tire, transform, vm, wash, weather, wheel, wing_element,
};
pub use xp_dataref::{commands, dataref, flight_map, keymap};
pub use xp_discord as discord;
pub use xp_obj as obj8;
pub use xp_scenery::{apt, install, world};
pub use xp_sim::{flight, pilot};

pub mod ui_resources;

pub mod menu_commands;

pub mod ui_state;

pub mod ui_input;

pub mod ui_focus;

pub mod keyboard_dispatch;

pub mod key_intercept;

pub mod xplm_key_hook;

pub mod xplm_context;

pub mod xplm_deferred;

pub mod xplm_sniffer;

pub mod ui_menu_key;

pub mod ui_menu_action;

pub mod ui_menu_direct;

pub mod ui_menu_close;

pub mod ui_window_order;

pub mod ui_window_visibility;

pub mod ui_widget_tree;

pub mod ui_window_graph;

pub mod ui_menu_close_graph;

pub mod ui_menu_direct_graph;

pub mod ui_menu_action_graph;

pub mod ui_menu_key_graph;

pub mod ui_focus_binding;

pub mod ui_menu_focus_graph;

pub mod ui_input_construct;

pub mod ui_panel_construct;

pub mod ui_menu_member_construct;

pub mod ui_interface_chain;

pub mod ui_main_menu_construct;

pub mod ui_callback_links;

pub mod ui_native_string;

pub mod ui_dialog_children;

pub mod ui_callback_list;

pub mod ui_dialog_child_context;

pub mod ui_context_refresh;

pub mod ui_dialog_child_selection;

pub mod ui_record_construct;

pub mod ui_dialog_child_root;

pub mod ui_dialog_child_weather;

pub mod ui_nested_refresh;

pub mod ui_payload_move;

pub mod ui_root_refresh;

pub mod ui_refresh_chain;

pub mod ui_engine_query;
pub mod ui_fifth_refresh;

pub mod ui_indexed_records;

pub mod ui_weather_refresh;

pub mod ui_selection_changed;

pub mod ui_selection_alternate;

pub mod ui_selection_reset_gate;
