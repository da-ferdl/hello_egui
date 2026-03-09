#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod route_kind;
mod router;
mod router_builder;
/// Transition types
pub mod transition;

use crate::transition::{ActiveTransition, SlideFadeTransition, SlideTransition, Transition};
use egui::emath::ease_in_ease_out;
use egui::{Ui, Vec2};
use std::any::Any;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;

pub use router::EguiRouter;
pub use router_builder::RouterBuilder;

/// A route instance created by a [`RouteHandler`]
pub trait Route<State = ()> {
    /// Render the route ui
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut State, route_arg: RouteArgument<'_>);
}

impl<F: FnMut(&mut Ui, &mut State, RouteArgument<'_>), State> Route<State> for F {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut State, route_arg: RouteArgument<'_>) {
        self(ui, state, route_arg);
    }
}

static ID: AtomicUsize = AtomicUsize::new(0);

struct RouteState<State> {
    path: String,
    route: RouteHandlerResult<Box<dyn Route<State>>>,
    route_arg: RouteArg,
    id: usize,
    state: u32,
}

/// Router Result type
pub type RouterResult<T = ()> = Result<T, RouterError>;

/// Router error
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    /// Not found error
    #[error("Route not found")]
    NotFound,
}

/// Request passed to a [`handler::MakeHandler`]
pub struct Request<'a, State = ()> {
    /// Optional argument passed to the request.
    pub arg: RouteArg,
    /// The custom state
    pub state: &'a mut State,
}

/// Internal type definition for a optional route argument.
pub(crate) type RouteArg = Option<Rc<dyn Any>>;

/// Holds a reference to the route argument that was given when navigating to the
/// regarding route, if an argument for the route was provided.
pub struct RouteArgument<'a>(pub &'a RouteArg);
impl<'a> RouteArgument<'a> {
    /// Returns a reference to the expected route argument type, if an argument
    /// of type `T` was set on navigation to the route.
    ///
    /// If no route argument was set on navigation or the argument is not of type `T`
    /// `None` is returned.
    pub fn get<T: 'static>(&self) -> Option<&'a T> {
        (self.0.as_ref()?).downcast_ref()
    }
}

/// Error returned from a [RouteHandler]
#[derive(Debug, thiserror::Error)]
pub enum RouteHandlerError {
    /// Not found error
    #[error("Page not found")]
    NotFound,
    /// Custom error message
    #[error("{0}")]
    Message(String),
    /// Boxed error
    #[error("Handler error: {0}")]
    Boxed(Box<dyn std::error::Error + Send + Sync>),
}

/// A [RouteHandler] Result type
pub type RouteHandlerResult<T = ()> = Result<T, RouteHandlerError>;

/// Handler for a route
pub type RouteHandler<State> =
    Box<dyn FnMut(Request<State>) -> RouteHandlerResult<Box<dyn Route<State>>>>;

/// Page transition configuration
#[derive(Debug, Clone)]
pub struct TransitionConfig {
    duration: Option<f32>,
    easing: fn(f32) -> f32,
    in_: Transition,
    out: Transition,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            duration: None,
            easing: ease_in_ease_out,
            in_: transition::SlideTransition::new(Vec2::X).into(),
            out: transition::SlideTransition::new(Vec2::X * -0.3).into(),
        }
    }
}

impl TransitionConfig {
    /// Create a new transition
    pub fn new(in_: impl Into<Transition>, out: impl Into<Transition>) -> Self {
        Self {
            in_: in_.into(),
            out: out.into(),
            ..Self::default()
        }
    }

    /// An iOS-like slide transition (Same as [`TransitionConfig::default`])
    pub fn slide() -> Self {
        Self::default()
    }

    /// An Android-like fade up transition
    pub fn fade_up() -> Self {
        Self::new(
            SlideFadeTransition(
                SlideTransition::new(Vec2::Y * 0.3),
                transition::FadeTransition,
            ),
            transition::NoTransition,
        )
    }

    /// A basic fade transition
    pub fn fade() -> Self {
        Self::new(transition::FadeTransition, transition::FadeTransition)
    }

    /// No transition
    pub fn none() -> Self {
        Self::new(transition::NoTransition, transition::NoTransition)
    }

    /// Customise the easing function
    pub fn with_easing(mut self, easing: fn(f32) -> f32) -> Self {
        self.easing = easing;
        self
    }

    /// Customise the duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = Some(duration);
        self
    }
}

struct CurrentTransition<State> {
    active_transition: ActiveTransition,
    leaving_route: Option<RouteState<State>>,
}
