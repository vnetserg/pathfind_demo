use crate::scene::PathtfindScene;
use crate::pathfind::find_and_render_path;

use wasm_bindgen::JsCast;

use std::rc::Rc;
use std::cell::RefCell;

////////////////////////////////////////////////////////////////////////////////

pub fn init(scene: Rc<RefCell<PathtfindScene>>) {
    let text_code = get_html_element("text-code");
    let text_output = get_html_element("text-output");
    let button_run = get_html_element("button-run");

    let ui_manager = Box::leak(Box::new(UiManager { scene, text_code, text_output, button_run }));
    ui_manager.init_callbacks();
}

fn get_html_element<T: JsCast + Clone>(name: &str) -> T {
    let window = web_sys::window().expect("global window does not exists");    
    let document = window.document().expect("expecting a document on window");
    document
        .get_element_by_id(name)
        .ok_or_else(|| panic!("should have a '{}' element on the page", name))
        .unwrap()
        .dyn_ref::<T>()
        .ok_or_else(|| panic!("'{}' element is of wrong type", name))
        .unwrap()
        .clone()
}

////////////////////////////////////////////////////////////////////////////////

struct UiManager {
    scene: Rc<RefCell<PathtfindScene>>,
    text_code: web_sys::HtmlTextAreaElement,
    text_output: web_sys::HtmlTextAreaElement,
    button_run: web_sys::HtmlButtonElement,
}

impl UiManager {
    fn init_callbacks(&'static self) {
        let on_run_clicked = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            self.on_run_clicked()
        }) as Box<dyn Fn()>);
        self.button_run.set_onclick(Some(on_run_clicked.as_ref().unchecked_ref()));
        on_run_clicked.forget();
    }

    fn on_run_clicked(&self) {
        self.text_output.set_value("");
        let mut scene = self.scene.borrow_mut();

        let maybe_draw_commands = find_and_render_path(
            &self.text_code.value(),
            scene.grid(),
            scene.start(),
            scene.finish(),
        );

        match maybe_draw_commands {
            Ok(draw_commands) => scene.set_draw_commands(draw_commands),
            Err(traceback) => self.text_output.set_value(&traceback),
        }
    }
}
