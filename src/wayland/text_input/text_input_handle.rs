use std::sync::{Arc, Mutex};

use wayland_protocols::wp::text_input::zv3::server::zwp_text_input_v3::{self, ZwpTextInputV3};
use wayland_server::backend::{ClientId, ObjectId};
use wayland_server::{protocol::wl_surface::WlSurface, Dispatch, Resource};

use crate::utils::IsAlive;
use crate::wayland::input_method::InputMethodHandle;

use super::TextInputManagerState;

#[derive(Debug)]
struct Instance {
    instance: ZwpTextInputV3,
    serial: u32,
    ready: bool,
}

#[derive(Default, Debug)]
pub(crate) struct TextInput {
    instances: Vec<Instance>,
    focus: Option<WlSurface>,
}

impl TextInput {
    fn with_focused_text_input<F>(&mut self, mut f: F)
    where
        F: FnMut(&ZwpTextInputV3, &WlSurface, u32, &mut bool),
    {
        if let Some(ref surface) = self.focus {
            if !surface.alive() {
                return;
            }
            for ti in self.instances.iter_mut() {
                if ti.instance.id().same_client_as(&surface.id()) {
                    f(&ti.instance, surface, ti.serial, &mut ti.ready);
                }
            }
        }
    }
}

/// Handle to text input instances
#[derive(Default, Debug, Clone)]
pub struct TextInputHandle {
    pub(crate) inner: Arc<Mutex<TextInput>>,
}

impl TextInputHandle {
    pub(super) fn add_instance(&self, instance: &ZwpTextInputV3) {
        let mut inner = self.inner.lock().unwrap();
        inner.instances.push(Instance {
            instance: instance.clone(),
            serial: 0,
            ready: false,
        });
    }

    fn increment_serial(&self, text_input: &ZwpTextInputV3) {
        let mut inner = self.inner.lock().unwrap();
        for ti in inner.instances.iter_mut() {
            if &ti.instance == text_input {
                ti.ready = true;
                ti.serial += 1;
            }
        }
    }

    pub(crate) fn focus(&self) -> Option<WlSurface> {
        self.inner.lock().unwrap().focus.clone()
    }

    pub(crate) fn leave(&self, surface: &WlSurface) {
        let inner = self.inner.lock().unwrap();
        for ti in inner.instances.iter() {
            if ti.instance.id().same_client_as(&surface.id()) {
                ti.instance.leave(surface);
            }
        }
    }

    pub(crate) fn enter(&self, surface: &WlSurface) {
        let mut inner = self.inner.lock().unwrap();
        inner.focus = Some(surface.clone());
        for ti in inner.instances.iter() {
            if ti.instance.id().same_client_as(&surface.id()) {
                ti.instance.enter(surface);
            }
        }
    }

    pub(crate) fn done(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.with_focused_text_input(|ti, _, serial, ready| {
            if *ready {
                *ready = false;
                ti.done(serial);
            }
        });
    }

    /// Callback function to use on the current focused text input surface
    pub(crate) fn with_focused_text_input<F>(&self, mut f: F)
    where
        F: FnMut(&ZwpTextInputV3, &WlSurface),
    {
        let mut inner = self.inner.lock().unwrap();
        inner.with_focused_text_input(|ti, surface, _, _| {
            f(ti, surface);
        });
    }

    pub(crate) fn focused_text_input_serial<F>(&self, mut f: F)
    where
        F: FnMut(u32),
    {
        let mut inner = self.inner.lock().unwrap();
        inner.with_focused_text_input(|_, _, serial, _| {
            f(serial);
        });
    }
}

/// User data of ZwpTextInputV3 object
#[derive(Debug)]
pub struct TextInputUserData {
    pub(super) handle: TextInputHandle,
    pub(crate) input_method_handle: InputMethodHandle,
}

impl<D> Dispatch<ZwpTextInputV3, TextInputUserData, D> for TextInputManagerState
where
    D: Dispatch<ZwpTextInputV3, TextInputUserData>,
    D: 'static,
{
    fn request(
        _state: &mut D,
        _client: &wayland_server::Client,
        resource: &ZwpTextInputV3,
        request: zwp_text_input_v3::Request,
        data: &TextInputUserData,
        _dhandle: &wayland_server::DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, D>,
    ) {
        match request {
            zwp_text_input_v3::Request::Enable => {
                // To avoid keeping uneccessary state in the compositor the events are not double buffered,
                // hence this request is unused
            }
            zwp_text_input_v3::Request::Disable => {
                // To avoid keeping uneccessary state in the compositor the events are not double buffered,
                // hence this request is unused
            }
            zwp_text_input_v3::Request::SetSurroundingText { text, cursor, anchor } => {
                data.input_method_handle.with_instance(|input_method| {
                    input_method.surrounding_text(text.clone(), cursor as u32, anchor as u32)
                });
            }
            zwp_text_input_v3::Request::SetTextChangeCause { cause } => {
                data.input_method_handle.with_instance(|input_method| {
                    input_method.text_change_cause(cause.into_result().unwrap())
                });
            }
            zwp_text_input_v3::Request::SetContentType { hint, purpose } => {
                data.input_method_handle.with_instance(|input_method| {
                    input_method.content_type(hint.into_result().unwrap(), purpose.into_result().unwrap());
                });
            }
            zwp_text_input_v3::Request::SetCursorRectangle { x, y, width, height } => {
                data.input_method_handle
                    .set_text_input_rectangle(x, y, width, height);
            }
            zwp_text_input_v3::Request::Commit => {
                data.handle.increment_serial(resource);
                data.input_method_handle.with_instance(|input_method| {
                    input_method.done();
                });
            }
            zwp_text_input_v3::Request::Destroy => {
                // Nothing to do
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(_state: &mut D, _client: ClientId, ti: ObjectId, data: &TextInputUserData) {
        // Ensure IME is deactivated when text input dies.
        data.input_method_handle.with_instance(|input_method| {
            input_method.deactivate();
            input_method.done();
        });

        data.handle
            .inner
            .lock()
            .unwrap()
            .instances
            .retain(|i| i.instance.id() != ti);
    }
}
