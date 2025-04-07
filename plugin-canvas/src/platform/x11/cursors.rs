use x11rb::{cursor::Handle, protocol::xproto::Cursor, resource_manager, xcb_ffi::XCBConnection};

#[derive(Clone, Debug)]
pub struct Cursors {
    pub alias: Cursor,
    pub all_scroll: Cursor,
    pub arrow: Cursor,
    pub cell: Cursor,
    pub col_resize: Cursor,
    pub context_menu: Cursor,
    pub copy: Cursor,
    pub crosshair: Cursor,
    pub e_resize: Cursor,
    pub ew_resize: Cursor,
    pub grab: Cursor,
    pub grabbing: Cursor,
    pub help: Cursor,
    pub r#move: Cursor,
    pub n_resize: Cursor,
    pub ne_resize: Cursor,
    pub nesw_resize: Cursor,
    pub no_drop: Cursor,
    pub not_allowed: Cursor,
    pub ns_resize: Cursor,
    pub nw_resize: Cursor,
    pub nwse_resize: Cursor,
    pub pointer: Cursor,
    pub progress: Cursor,
    pub row_resize: Cursor,
    pub s_resize: Cursor,
    pub se_resize: Cursor,
    pub sw_resize: Cursor,
    pub text: Cursor,
    pub vertical_text: Cursor,
    pub w_resize: Cursor,
    pub wait: Cursor,
    pub zoom_in: Cursor,
    pub zoom_out: Cursor,
}

impl Cursors {
    pub fn new(connection: &XCBConnection, screen: usize) -> Self {
        let database = resource_manager::new_from_default(connection).unwrap();
        let handle = Handle::new(connection, screen, &database)
            .unwrap()
            .reply()
            .unwrap();

        Self {
            alias: handle.load_cursor(connection, "alias").unwrap(),
            all_scroll: handle.load_cursor(connection, "all-scroll").unwrap(),
            arrow: handle.load_cursor(connection, "arrow").unwrap(),
            cell: handle.load_cursor(connection, "cell").unwrap(),
            col_resize: handle.load_cursor(connection, "col-resize").unwrap(),
            context_menu: handle.load_cursor(connection, "context-menu").unwrap(),
            copy: handle.load_cursor(connection, "copy").unwrap(),
            crosshair: handle.load_cursor(connection, "crosshair").unwrap(),
            e_resize: handle.load_cursor(connection, "e-resize").unwrap(),
            ew_resize: handle.load_cursor(connection, "ew-resize").unwrap(),
            grab: handle.load_cursor(connection, "grab").unwrap(),
            grabbing: handle.load_cursor(connection, "grabbing").unwrap(),
            help: handle.load_cursor(connection, "help").unwrap(),
            r#move: handle.load_cursor(connection, "move").unwrap(),
            n_resize: handle.load_cursor(connection, "n-resize").unwrap(),
            ne_resize: handle.load_cursor(connection, "ne-resize").unwrap(),
            nesw_resize: handle.load_cursor(connection, "nesw-resize").unwrap(),
            no_drop: handle.load_cursor(connection, "no-drop").unwrap(),
            not_allowed: handle.load_cursor(connection, "not-allowed").unwrap(),
            ns_resize: handle.load_cursor(connection, "ns-resize").unwrap(),
            nw_resize: handle.load_cursor(connection, "nw-resize").unwrap(),
            nwse_resize: handle.load_cursor(connection, "nwse-resize").unwrap(),
            pointer: handle.load_cursor(connection, "pointer").unwrap(),
            progress: handle.load_cursor(connection, "progress").unwrap(),
            row_resize: handle.load_cursor(connection, "row-resize").unwrap(),
            s_resize: handle.load_cursor(connection, "s-resize").unwrap(),
            se_resize: handle.load_cursor(connection, "se-resize").unwrap(),
            sw_resize: handle.load_cursor(connection, "sw-resize").unwrap(),
            text: handle.load_cursor(connection, "text").unwrap(),
            vertical_text: handle.load_cursor(connection, "vertical-text").unwrap(),
            w_resize: handle.load_cursor(connection, "w-resize").unwrap(),
            wait: handle.load_cursor(connection, "wait").unwrap(),
            zoom_in: handle.load_cursor(connection, "zoom-in").unwrap(),
            zoom_out: handle.load_cursor(connection, "zoom-out").unwrap(),
        }
    }
}
