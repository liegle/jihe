use std::{env, fs, io::Write, mem, panic, path::PathBuf, str::FromStr, sync::Arc};

use crate::{config::Config, parse::Parse, render::Render, state::State};

mod config;
mod parse;
mod render;
mod schedule;
mod state;

fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stderr)
        .format(log_format)
        .init();

    let args = env::args();
    if args.len() != 2 {
        println!("Usage: jihe <path>");
        return;
    }
    let path = args.last().unwrap();
    if !fs::exists(&path).is_ok_and(|b| b) {
        println!("File at {} not found", &path);
        return;
    }

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = App::Uninitialized { path };
    event_loop.run_app(&mut app).unwrap();
}

enum App {
    Uninitialized {
        path: String,
    },
    Ready {
        state: State,
        window: Arc<winit::window::Window>,
        parse: Parse,
        render: Render,
    },
    Dead,
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let App::Uninitialized { path } = self else {
            log::info!("Resumed but app was already inited");
            return;
        };

        let config = Config::default();
        let content = jihe_shared::Content::new();
        let scene = jihe_render::Scene::new(content);
        let state = State::new(config, scene.clone());

        let window = match event_loop.create_window(Default::default()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Can't create window because:{e}");
                return;
            }
        };
        window.set_title("jihe");
        let window = Arc::new(window);

        let parse = match Parse::new(PathBuf::from_str(&path).unwrap(), scene.clone(), {
            let window = window.clone();
            move || {
                window.request_redraw();
            }
        }) {
            Some(r) => r,
            None => {
                log::error!("Can't create parse");
                return;
            }
        };

        let render = match Render::new(
            scene,
            window.clone(),
            state.config.render_per_sec,
            state.config.resize_per_sec,
        ) {
            Some(r) => r,
            None => {
                log::error!("Can't create render");
                return;
            }
        };

        *self = App::Ready {
            state,
            window,
            parse,
            render,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Ready {
            state,
            window,
            parse,
            render,
        } = self
        else {
            return;
        };
        use winit::event::WindowEvent;
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Exit");
                render.exit();
                parse.exit();
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => render.draw(),
            WindowEvent::Resized(size) => render.resize(size.into()),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if state.handle_keyboard_input(&event) {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                if state.handle_cursor_moved(&position) {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state: elem_state,
                button,
            } => {
                use winit::window::{Cursor, CursorIcon};
                if state.handle_mouse_input(&elem_state, &button) {
                    window.set_cursor(Cursor::Icon(CursorIcon::Grabbing));
                } else {
                    window.set_cursor(Cursor::Icon(CursorIcon::Default));
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase,
            } => {
                if state.handle_mouse_wheel(&delta, &phase) {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { parse, render, .. } = mem::replace(self, App::Dead) {
            if let Err(e) = render.join() {
                log::error!("Render thread is found panicked when exiting");
                panic::resume_unwind(e);
            }
            if let Err(e) = parse.join() {
                log::error!("Parse thread is found panicked when exiting");
                panic::resume_unwind(e);
            }
        }
    }
}

fn log_format(
    buf: &mut env_logger::fmt::Formatter,
    record: &log::Record<'_>,
) -> Result<(), std::io::Error> {
    use env_logger::fmt::style::{AnsiColor, Color, Style};

    const STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));

    const TRACE_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    const DEBUG_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Blue)));
    const INFO_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    const WARN_STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
    const ERROR_STYLE: Style = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold();

    let time = chrono::Local::now().format("%F %T%.3f");
    let level = record.level();
    let path = record.module_path_static().unwrap_or("???");
    let line = record.line().unwrap_or(u32::MAX);
    let args = record.args();

    let style = match level {
        log::Level::Trace => TRACE_STYLE,
        log::Level::Debug => DEBUG_STYLE,
        log::Level::Info => INFO_STYLE,
        log::Level::Warn => WARN_STYLE,
        log::Level::Error => ERROR_STYLE,
    };

    writeln!(
        buf,
        "[{STYLE}{time}{STYLE:#} {style}{level}{style:#} {STYLE}{path}#{line}{STYLE:#}] {args}",
    )
}
