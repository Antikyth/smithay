use wayland_protocols_misc::zwp_input_method_v2::server::zwp_input_popup_surface_v2::{
    self, ZwpInputPopupSurfaceV2,
};
use wayland_server::{backend::ClientId, protocol::wl_surface::WlSurface, Dispatch, Resource};

use crate::utils::{
    alive_tracker::{AliveTracker, IsAlive},
    Physical, Rectangle,
};
use crate::utils::{Logical, Point};

use super::InputMethodManagerState;

/// Handle to a popup surface
#[derive(Debug, Clone, Default)]
pub struct PopupHandle {
    pub surface: Option<PopupSurface>,
    pub rectangle: Rectangle<i32, Physical>,
}

/// A handle to an input method popup surface
#[derive(Debug, Clone)]
pub struct PopupSurface {
    /// The surface role for the input method popup
    pub surface_role: ZwpInputPopupSurfaceV2,
    surface: WlSurface,
    parent: WlSurface,
    /// Rectangle with position and size of  text cursor, used for placement of popup surface
    pub rectangle: Rectangle<i32, Physical>,
    parent_location: Rectangle<i32, Logical>,
}

impl std::cmp::PartialEq for PopupSurface {
    fn eq(&self, other: &Self) -> bool {
        self.surface_role == other.surface_role
    }
}

impl PopupSurface {
    pub(crate) fn new(
        surface_role: ZwpInputPopupSurfaceV2,
        surface: WlSurface,
        parent: WlSurface,
        rectangle: Rectangle<i32, Physical>,
        parent_location: Rectangle<i32, Logical>,
    ) -> Self {
        Self {
            surface_role,
            surface,
            parent,
            rectangle,
            parent_location,
        }
    }

    /// Is the input method popup surface referred by this handle still alive?
    pub fn alive(&self) -> bool {
        // TODO other things to check? This may not sufice.
        let role_data: &InputMethodPopupSurfaceUserData = self.surface_role.data().unwrap();
        self.surface.alive() && role_data.alive_tracker.alive()
    }

    /// Access to the underlying `wl_surface` of this popup
    pub fn wl_surface(&self) -> &WlSurface {
        &self.surface
    }

    /// Access to the parent surface associated with this popup
    pub fn get_parent_surface(&self) -> WlSurface {
        self.parent.clone()
    }

    /// Access to the parent surface location associated with this popup
    pub fn parent_location(&self) -> Rectangle<i32, Logical> {
        self.parent_location
    }

    /// Used to access the location of an input popup surface relative to the parent
    pub fn location(&self) -> Point<i32, Logical> {
        Point::from((
            self.rectangle.loc.x - self.rectangle.size.w,
            self.rectangle.loc.y + self.rectangle.size.h,
        ))
    }

    /// Set relative location of text cursor
    pub fn set_rectangle(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.rectangle = Rectangle::from_loc_and_size((x, y), (width, height));
        self.surface_role.text_input_rectangle(x, y, width, height);
    }
}

/// User data of ZwpInputPopupSurfaceV2 object
#[derive(Debug)]
pub struct InputMethodPopupSurfaceUserData {
    pub(super) alive_tracker: AliveTracker,
}

impl<D> Dispatch<ZwpInputPopupSurfaceV2, InputMethodPopupSurfaceUserData, D> for InputMethodManagerState {
    fn request(
        _state: &mut D,
        _client: &wayland_server::Client,
        _resource: &ZwpInputPopupSurfaceV2,
        request: zwp_input_popup_surface_v2::Request,
        _data: &InputMethodPopupSurfaceUserData,
        _dhandle: &wayland_server::DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, D>,
    ) {
        match request {
            zwp_input_popup_surface_v2::Request::Destroy => {
                // Nothing to do
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(
        _state: &mut D,
        _client: ClientId,
        _object: &ZwpInputPopupSurfaceV2,
        data: &InputMethodPopupSurfaceUserData,
    ) {
        data.alive_tracker.destroy_notify();
    }
}
