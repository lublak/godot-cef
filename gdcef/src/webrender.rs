use cef::{self, rc::Rc, *, sys::cef_cursor_type_t};
use cef_app::CursorType;
use godot::global::godot_print;
use std::sync::{Arc, Mutex};

fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for chunk in bgra.chunks_exact(4) {
        rgba.push(chunk[2]);
        rgba.push(chunk[1]);
        rgba.push(chunk[0]);
        rgba.push(chunk[3]);
    }
    rgba
}

wrap_render_handler! {
    pub struct RenderHandlerBuilder {
        handler: cef_app::OsrRenderHandler,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                if let Ok(size) = self.handler.size.lock() {
                    if size.width > 0.0 && size.height > 0.0 {
                        rect.width = size.width as _;
                        rect.height = size.height as _;
                    }
                }
            }
        }

        fn screen_info(
            &self,
            _browser: Option<&mut Browser>,
            screen_info: Option<&mut ScreenInfo>,
        ) -> ::std::os::raw::c_int {
            if let Some(screen_info) = screen_info {
                if let Ok(scale) = self.handler.device_scale_factor.lock() {
                    screen_info.device_scale_factor = *scale;
                    return true as _;
                }
            }
            false as _
        }

        fn screen_point(
            &self,
            _browser: Option<&mut Browser>,
            _view_x: ::std::os::raw::c_int,
            _view_y: ::std::os::raw::c_int,
            _screen_x: Option<&mut ::std::os::raw::c_int>,
            _screen_y: Option<&mut ::std::os::raw::c_int>,
        ) -> ::std::os::raw::c_int {
            false as _
        }

        fn on_accelerated_paint(
            &self,
            _browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            _info: Option<&AcceleratedPaintInfo>,
        ) {
            godot_print!("on_accelerated_paint, type: {:?}", type_);
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            _type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: ::std::os::raw::c_int,
            height: ::std::os::raw::c_int,
        ) {
            if buffer.is_null() || width <= 0 || height <= 0 {
                return;
            }

            let width = width as u32;
            let height = height as u32;
            let buffer_size = (width * height * 4) as usize;
            let bgra_data = unsafe { std::slice::from_raw_parts(buffer, buffer_size) };
            let rgba_data = bgra_to_rgba(bgra_data);

            if let Ok(mut frame_buffer) = self.handler.frame_buffer.lock() {
                frame_buffer.update(rgba_data, width, height);
            }
        }
    }
}

impl RenderHandlerBuilder {
    pub fn build(handler: cef_app::OsrRenderHandler) -> RenderHandler {
        Self::new(handler)
    }
}

fn cef_cursor_to_cursor_type(cef_type: cef::sys::cef_cursor_type_t) -> CursorType {
    match cef_type {
        cef_cursor_type_t::CT_POINTER => CursorType::Arrow,
        cef_cursor_type_t::CT_IBEAM => CursorType::IBeam,
        cef_cursor_type_t::CT_HAND => CursorType::Hand,
        cef_cursor_type_t::CT_CROSS => CursorType::Cross,
        cef_cursor_type_t::CT_WAIT => CursorType::Wait,
        cef_cursor_type_t::CT_HELP => CursorType::Help,
        cef_cursor_type_t::CT_MOVE => CursorType::Move,
        cef_cursor_type_t::CT_NORTHRESIZE
        | cef_cursor_type_t::CT_SOUTHRESIZE
        | cef_cursor_type_t::CT_NORTHSOUTHRESIZE => CursorType::ResizeNS,
        cef_cursor_type_t::CT_EASTRESIZE
        | cef_cursor_type_t::CT_WESTRESIZE
        | cef_cursor_type_t::CT_EASTWESTRESIZE => CursorType::ResizeEW,
        cef_cursor_type_t::CT_NORTHEASTRESIZE
        | cef_cursor_type_t::CT_SOUTHWESTRESIZE
        | cef_cursor_type_t::CT_NORTHEASTSOUTHWESTRESIZE => CursorType::ResizeNESW,
        cef_cursor_type_t::CT_NORTHWESTRESIZE
        | cef_cursor_type_t::CT_SOUTHEASTRESIZE
        | cef_cursor_type_t::CT_NORTHWESTSOUTHEASTRESIZE => CursorType::ResizeNWSE,
        cef_cursor_type_t::CT_NOTALLOWED => CursorType::NotAllowed,
        cef_cursor_type_t::CT_PROGRESS => CursorType::Progress,
        _ => CursorType::Arrow,
    }
}

wrap_display_handler! {
    pub(crate) struct DisplayHandlerBuilder {
        cursor_type: Arc<Mutex<CursorType>>,
    }

    impl DisplayHandler {
        fn on_cursor_change(
            &self,
            _browser: Option<&mut Browser>,
            _cursor: *mut u8,
            type_: cef::CursorType,
            _custom_cursor_info: Option<&CursorInfo>,
        ) -> i32 {
            let cursor = cef_cursor_to_cursor_type(type_.into());
            if let Ok(mut ct) = self.cursor_type.lock() {
                *ct = cursor;
            }
            false as i32
        }
    }
}

impl DisplayHandlerBuilder {
    pub fn build(cursor_type: Arc<Mutex<CursorType>>) -> DisplayHandler {
        Self::new(cursor_type)
    }
}

wrap_context_menu_handler! {
    pub(crate) struct ContextMenuHandlerBuilder {}

    impl ContextMenuHandler {
        fn on_before_context_menu(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _params: Option<&mut ContextMenuParams>,
            model: Option<&mut MenuModel>,
        ) {
            if let Some(model) = model {
                model.clear();
            }
        }
    }
}

impl ContextMenuHandlerBuilder {
    pub fn build() -> ContextMenuHandler {
        Self::new()
    }
}

wrap_client! {
    pub(crate) struct ClientBuilder {
        render_handler: RenderHandler,
        display_handler: DisplayHandler,
        context_menu_handler: ContextMenuHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<cef::RenderHandler> {
            Some(self.render_handler.clone())
        }

        fn display_handler(&self) -> Option<cef::DisplayHandler> {
            Some(self.display_handler.clone())
        }

        fn context_menu_handler(&self) -> Option<cef::ContextMenuHandler> {
            Some(self.context_menu_handler.clone())
        }
    }
}

impl ClientBuilder {
    pub(crate) fn build(render_handler: cef_app::OsrRenderHandler) -> Client {
        let cursor_type = render_handler.get_cursor_type();
        Self::new(
            RenderHandlerBuilder::build(render_handler),
            DisplayHandlerBuilder::build(cursor_type),
            ContextMenuHandlerBuilder::build(),
        )
    }
}

#[derive(Clone)]
pub struct OsrRequestContextHandler {}

wrap_request_context_handler! {
    pub(crate) struct RequestContextHandlerBuilder {
        handler: OsrRequestContextHandler,
    }

    impl RequestContextHandler {}
}

impl RequestContextHandlerBuilder {
    pub(crate) fn build(handler: OsrRequestContextHandler) -> RequestContextHandler {
        Self::new(handler)
    }
}
