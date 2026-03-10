use crate::route_kind::RouteKind;
use crate::{EguiRouter, MakeHandler, TransitionConfig};
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

/// Builder to create a [`EguiRouter`]
pub struct RouterBuilder<State> {
    pub(crate) routes: HashMap<String, RouteKind<State>>,
    pub(crate) initial_route: (String, Option<Box<dyn Any>>),

    pub(crate) forward_transition: TransitionConfig,
    pub(crate) backward_transition: TransitionConfig,
    pub(crate) replace_transition: TransitionConfig,

    pub(crate) default_duration: Option<f32>,

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
    pub fn route<Han: MakeHandler<State> + 'static>(
        mut self,
        route: &str,
        mut handler: Han,
    ) -> Self {
        self.routes.insert(
            route.into(),
            RouteKind::Route(Box::new(move || handler.handle())),
        );
        self
    }

    /// Add a set of routes at once.
    pub fn routes<Han: MakeHandler<State> + 'static>(mut self, routes: Vec<(&str, Han)>) -> Self {
        for mut r in routes {
            self.routes
                .insert(r.0.into(), RouteKind::Route(Box::new(move || r.1.handle())));
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
