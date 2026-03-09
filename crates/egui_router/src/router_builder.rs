use crate::route_kind::RouteKind;
use crate::{EguiRouter, TransitionConfig};
use crate::{RouteHandler, RouteHandlerError};
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) type ErrorUi<State> =
    Arc<Box<dyn Fn(&mut egui::Ui, &State, &RouteHandlerError) + Send + Sync>>;
pub(crate) type LoadingUi<State> = Arc<Box<dyn Fn(&mut egui::Ui, &State) + Send + Sync>>;

/// Builder to create a [`EguiRouter`]
pub struct RouterBuilder<State> {
    pub(crate) routes: HashMap<String, RouteKind<State>>,
    pub(crate) initial_route: (String, Option<Box<dyn Any>>),

    pub(crate) forward_transition: TransitionConfig,
    pub(crate) backward_transition: TransitionConfig,
    pub(crate) replace_transition: TransitionConfig,

    pub(crate) default_duration: Option<f32>,

    pub(crate) error_ui: ErrorUi<State>,
    pub(crate) loading_ui: LoadingUi<State>,

    pub(crate) swipe_back_gesture_enabled: bool,
    pub(crate) swipe_back_edge_width: f32,
    pub(crate) swipe_back_threshold: f32,
}

/*impl<State: 'static> Default for RouterBuilder<State> {
    fn default() -> Self {
        Self::new("", None)
    }
}*/

impl<State: 'static> RouterBuilder<State> {
    /// Create a new router builder.
    ///
    /// - `initial_route_path` - The initial active route to show when the app starts.
    /// - `initial_route_arg` - Optional argument for the initial route.
    pub fn new(
        initial_route_path: impl Into<String>,
        initial_route_arg: Option<Box<dyn Any>>,
    ) -> Self {
        Self {
            routes: HashMap::new(),
            initial_route: (initial_route_path.into(), initial_route_arg),
            forward_transition: TransitionConfig::default(),
            backward_transition: TransitionConfig::default(),
            replace_transition: TransitionConfig::fade(),
            default_duration: None,
            error_ui: Arc::new(Box::new(|ui, _, err| {
                ui.label(format!("Error: {err}"));
            })),
            loading_ui: Arc::new(Box::new(|ui, _| {
                ui.spinner();
            })),
            swipe_back_gesture_enabled: false,
            swipe_back_edge_width: 40.0,
            swipe_back_threshold: 0.4,
        }
    }

    /// Set the transition for both forward and backward transitions
    pub fn transition(mut self, transition: TransitionConfig) -> Self {
        self.forward_transition = transition.clone();
        self.backward_transition = transition;
        self
    }

    /// Set the transition for forward transitions
    pub fn forward_transition(mut self, transition: TransitionConfig) -> Self {
        self.forward_transition = transition;
        self
    }

    /// Set the transition for backward transitions
    pub fn backward_transition(mut self, transition: TransitionConfig) -> Self {
        self.backward_transition = transition;
        self
    }

    /// Set the transition for replace transitions
    pub fn replace_transition(mut self, transition: TransitionConfig) -> Self {
        self.replace_transition = transition;
        self
    }

    /// Set the default duration for transitions
    pub fn default_duration(mut self, duration: f32) -> Self {
        self.default_duration = Some(duration);
        self
    }

    /// Set the error UI
    /// Call this *before* you call `.async_route()`, otherwise the error UI will not be used in async routes.
    pub fn error_ui(
        mut self,
        f: impl Fn(&mut egui::Ui, &State, &RouteHandlerError) + 'static + Send + Sync,
    ) -> Self {
        self.error_ui = Arc::new(Box::new(f));
        self
    }

    /// Set the loading UI
    /// Call this *before* you call `.async_route()`, otherwise the loading UI will not be used in async routes.
    pub fn loading_ui(mut self, f: impl Fn(&mut egui::Ui, &State) + 'static + Send + Sync) -> Self {
        self.loading_ui = Arc::new(Box::new(f));
        self
    }

    /// Add a route. Check the [matchit] documentation for information about the route syntax.
    /// The handler will be called with [`crate::Request`] and should return a [Route].
    ///
    /// # Example
    /// ```rust
    /// # use egui::Ui;
    /// # use egui_router::{EguiRouter, HandlerError, HandlerResult, Request, Route};
    ///
    /// pub fn my_handler(_req: Request) -> impl Route {
    ///     |ui: &mut Ui, _: &mut ()| {
    ///         ui.label("Hello, world!");
    ///     }
    /// }
    ///
    /// pub fn my_fallible_handler(req: Request) -> HandlerResult<impl Route> {
    ///     let post = req.params.get("post").ok_or_else(|| HandlerError::NotFound)?.to_owned();
    ///     Ok(move |ui: &mut Ui, _: &mut ()| {
    ///        ui.label(format!("Post: {}", post));
    ///     })
    /// }
    ///
    /// let router: EguiRouter<()> = EguiRouter::builder()
    ///     .route("/", my_handler)
    ///     .route("/:post", my_fallible_handler)
    ///     .build(&mut ());
    pub fn route(mut self, route: &str, mut handler: RouteHandler<State>) -> Self {
        self.routes.insert(
            route.into(),
            RouteKind::Route(Box::new(move |req| handler(req))),
        );
        self
    }

    /// Add a set of routes at once.
    pub fn routes(mut self, routes: Vec<(&str, RouteHandler<State>)>) -> Self {
        for mut r in routes {
            self.routes.insert(
                r.0.into(),
                RouteKind::Route(Box::new(move |req| (r.1)(req))),
            );
        }

        self
    }

    /// Add a redirect route. Whenever this route matches, it'll redirect to the route you specified.
    pub fn route_redirect(
        mut self,
        route: &str,
        redirect_arg: Option<Box<dyn Any>>,
        redirect: impl Into<String>,
    ) -> Self {
        self.routes.insert(
            route.into(),
            RouteKind::Redirect(redirect.into(), redirect_arg.map(|v| Rc::from(v))),
        );
        self
    }

    /// Enable or disable the iOS-style swipe-to-go-back gesture (disabled by default)
    pub fn swipe_back_gesture(mut self, enabled: bool) -> Self {
        self.swipe_back_gesture_enabled = enabled;
        self
    }

    /// Set the edge width in pixels where the swipe gesture can be initiated (default: 40.0)
    pub fn swipe_back_edge_width(mut self, width: f32) -> Self {
        self.swipe_back_edge_width = width;
        self
    }

    /// Set the threshold (as a fraction of screen width) for completing the back navigation (default: 0.4)
    pub fn swipe_back_threshold(mut self, threshold: f32) -> Self {
        self.swipe_back_threshold = threshold;
        self
    }

    /// Build the router
    pub fn build(self, state: &mut State) -> EguiRouter<State> {
        EguiRouter::from_builder(self, state)
    }
}
