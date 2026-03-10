use crate::route_kind::RouteKind;
use crate::router_builder::RouterBuilder;
use crate::transition::{ActiveTransition, ActiveTransitionResult};
use crate::{
    CurrentTransition, RouteArg, RouteArgument, RouteState, RouterError, RouterResult,
    TransitionConfig, ID,
};
use egui::{scroll_area, Id, Sense, Ui};
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;

/// The state of the iOS-style swipe-to-go-back gesture
#[derive(Debug, Clone)]
enum SwipeBackGestureState {
    /// No gesture is happening
    Idle,
    /// User is actively swiping
    Swiping {
        /// Distance swiped in pixels
        distance: f32,
    },
    /// Gesture was cancelled due to vertical movement, wait for release
    Cancelled,
}

/// A router instance
pub struct EguiRouter<State> {
    routes: HashMap<String, RouteKind<State>>,

    history: Vec<RouteState<State>>,

    forward_transition: TransitionConfig,
    backward_transition: TransitionConfig,
    replace_transition: TransitionConfig,

    current_transition: Option<CurrentTransition<State>>,
    default_duration: Option<f32>,

    /// Enable iOS-style swipe-to-go-back gesture
    swipe_back_gesture_enabled: bool,
    /// Minimum distance from left edge to start the gesture (in pixels)
    swipe_back_edge_width: f32,
    /// Minimum swipe distance to trigger navigation (as fraction of screen width)
    swipe_back_threshold: f32,
}

impl<State: 'static> EguiRouter<State> {
    /// Create a new [`RouterBuilder`]
    ///
    /// - `initial_route_path` - The initial active route to show when the app starts.
    /// - `initial_route_arg` - Optional argument for the initial route.
    pub fn builder(
        initial_route_path: impl Into<String>,
        initial_route_arg: Option<Box<dyn Any>>,
    ) -> RouterBuilder<State> {
        RouterBuilder::new(initial_route_path, initial_route_arg)
    }

    pub(crate) fn from_builder(builder: RouterBuilder<State>, state: &mut State) -> Self {
        let mut router = Self {
            routes: builder.routes,
            history: Vec::new(),
            current_transition: None,
            forward_transition: builder.forward_transition,
            backward_transition: builder.backward_transition,
            replace_transition: builder.replace_transition,
            default_duration: builder.default_duration,
            swipe_back_gesture_enabled: builder.swipe_back_gesture_enabled,
            swipe_back_edge_width: builder.swipe_back_edge_width,
            swipe_back_threshold: builder.swipe_back_threshold,
        };

        let (path, arg) = builder.initial_route;

        router
            .navigate_impl(
                state,
                &path,
                arg.map(|v| Rc::from(v)),
                TransitionConfig::none(),
                0,
            )
            .unwrap();

        router
    }

    /// Get the active route
    pub fn active_route(&self) -> Option<(&str, RouteArgument<'_>)> {
        self.history
            .last()
            .map(|r| (r.path.as_str(), RouteArgument(&r.route_arg)))
    }

    /// How many history entries are there?
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Iterate over the paths in the history
    pub fn history(&self) -> impl Iterator<Item = &str> {
        self.history.iter().map(|s| s.path.as_str())
    }

    fn navigate_impl(
        &mut self,
        state: &mut State,
        path: &str,
        route_arg: RouteArg,
        transition_config: TransitionConfig,
        new_state: u32,
    ) -> RouterResult {
        let route_kind = self.routes.get_mut(path).ok_or(RouterError::NotFound)?;
        let mut redirect = None;

        match route_kind {
            RouteKind::Route(handler) => {
                let route = handler();
                self.history.push(RouteState {
                    path: path.into(),
                    route,
                    route_arg,
                    id: ID.fetch_add(1, Ordering::SeqCst),
                    state: new_state,
                });

                self.current_transition = Some(CurrentTransition {
                    active_transition: ActiveTransition::forward(transition_config.clone())
                        .with_default_duration(self.default_duration),
                    leaving_route: None,
                });
            }
            RouteKind::Redirect(r, a) => {
                redirect = Some((r.clone(), a.clone()));
            }
        };

        if let Some((path, arg)) = redirect {
            self.navigate_impl(state, &path, arg, transition_config, new_state)?;
        }

        Ok(())
    }

    /// Navigate with a custom transition
    pub fn navigate_transition(
        &mut self,
        state: &mut State,
        path: impl Into<String>,
        route_arg: Option<Box<dyn Any>>,
        transition_config: TransitionConfig,
    ) -> RouterResult {
        let path = path.into();
        let current_state = self.history.last().map_or(0, |r| r.state);
        let new_state = current_state + 1;
        self.navigate_impl(
            state,
            &path,
            route_arg.map(|v| Rc::from(v)),
            transition_config,
            new_state,
        )?;
        Ok(())
    }

    /// Navigate with the default transition
    pub fn navigate(
        &mut self,
        state: &mut State,
        route: impl Into<String>,
        route_arg: Option<Box<dyn Any>>,
    ) -> RouterResult {
        self.navigate_transition(state, route, route_arg, self.forward_transition.clone())
    }

    fn back_impl(&mut self, transition_config: TransitionConfig) {
        if self.history.len() > 1 {
            let leaving_route = self.history.pop();
            self.current_transition = Some(CurrentTransition {
                active_transition: ActiveTransition::backward(transition_config)
                    .with_default_duration(self.default_duration),
                leaving_route,
            });
        }
    }

    /// Go back with a custom transition
    pub fn back_transition(&mut self, transition_config: TransitionConfig) -> RouterResult {
        self.back_impl(transition_config);
        Ok(())
    }

    /// Go back with the default transition
    pub fn back(&mut self) -> RouterResult {
        self.back_transition(self.backward_transition.clone())
    }

    /// Replace the current route with a custom transition
    pub fn replace_transition(
        &mut self,
        state: &mut State,
        path: impl Into<String>,
        route_arg: Option<Box<dyn Any>>,
        transition_config: TransitionConfig,
    ) -> RouterResult {
        self.replace_transition_impl(
            state,
            path,
            route_arg.map(|v| Rc::from(v)),
            transition_config,
        )
    }

    fn replace_transition_impl(
        &mut self,
        state: &mut State,
        path: impl Into<String>,
        route_arg: RouteArg,
        transition_config: TransitionConfig,
    ) -> RouterResult {
        let path = path.into();

        let route_kind = self.routes.get_mut(&path).ok_or(RouterError::NotFound)?;
        let mut redirect = None;

        let current_state = self.history.last().map_or(0, |r| r.state);
        let new_state = current_state;

        match route_kind {
            RouteKind::Route(handler) => {
                let leaving_route = self.history.pop();
                let route = handler();
                self.history.push(RouteState {
                    path: path,
                    route,
                    route_arg,
                    id: ID.fetch_add(1, Ordering::SeqCst),
                    state: new_state,
                });

                self.current_transition = Some(CurrentTransition {
                    active_transition: ActiveTransition::forward(transition_config.clone())
                        .with_default_duration(self.default_duration),
                    leaving_route,
                });
            }
            RouteKind::Redirect(r, a) => {
                redirect = Some((r.clone(), a.clone()));
            }
        };

        if let Some((path, arg)) = redirect {
            self.replace_transition_impl(state, path, arg, transition_config)?;
        }

        Ok(())
    }

    /// Replace the current route with the default transition
    pub fn replace(
        &mut self,
        state: &mut State,
        path: impl Into<String>,
        route_arg: Option<Box<dyn Any>>,
    ) -> RouterResult {
        self.replace_transition(state, path, route_arg, self.replace_transition.clone())
    }

    /// Render the router
    pub fn ui(&mut self, ui: &mut Ui, state: &mut State) {
        // Handle iOS-style swipe-to-go-back gesture
        if self.swipe_back_gesture_enabled && self.history.len() > 1 {
            self.handle_swipe_gesture(ui);
        }

        if let Some((last, previous)) = self.history.split_last_mut() {
            let result = if let Some(transition) = &mut self.current_transition {
                let leaving_route_state = transition.leaving_route.as_mut().or(previous.last_mut());
                Some(transition.active_transition.show(
                    ui,
                    state,
                    (last.id, RouteArgument(&last.route_arg), |ui, state, arg| {
                        last.route.ui(ui, state, arg)
                    }),
                    leaving_route_state.map(|r| {
                        (
                            r.id,
                            RouteArgument(&r.route_arg),
                            |ui: &mut Ui, state: &mut _, arg: RouteArgument| {
                                r.route.ui(ui, state, arg)
                            },
                        )
                    }),
                ))
            } else {
                ActiveTransition::show_default(ui, last.id, |ui| {
                    last.route.ui(ui, state, RouteArgument(&last.route_arg))
                });
                None
            };

            match result {
                Some(ActiveTransitionResult::Done) => {
                    self.current_transition = None;
                }
                Some(ActiveTransitionResult::Continue) | None => {}
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_swipe_gesture(&mut self, ui: &mut Ui) {
        let gesture_id = Id::new("router_swipe_back_gesture");

        // Get or create gesture state
        let last_state = ui.data_mut(|data| {
            data.get_temp_mut_or(gesture_id, SwipeBackGestureState::Idle)
                .clone()
        });

        let mut gesture_state = last_state;

        // Get the content rect for interaction
        let content_rect = ui.available_rect_before_wrap();
        let sense = ui.interact(content_rect, gesture_id, Sense::hover());

        // Check if there's something blocking the drag (e.g., scroll area)
        let is_something_blocking_drag = ui.ctx().dragged_id().is_some_and(|id| {
            // Ignore if the dragged id is a scroll area
            scroll_area::State::load(ui.ctx(), id).is_some()
        }) && !ui.ctx().is_being_dragged(gesture_id);

        if sense.contains_pointer() && !is_something_blocking_drag {
            let (pointer_pos, delta, any_released, velocity) = ui.input(|input| {
                (
                    input.pointer.interact_pos(),
                    if input.pointer.is_decidedly_dragging() {
                        Some(input.pointer.delta())
                    } else {
                        None
                    },
                    input.pointer.any_released(),
                    input.pointer.velocity(),
                )
            });

            if let Some(delta) = delta {
                match gesture_state {
                    SwipeBackGestureState::Idle => {
                        // Check if the gesture started from the left edge
                        if let Some(pos) = pointer_pos {
                            if pos.x <= content_rect.min.x + self.swipe_back_edge_width {
                                // Cancel if velocity is more vertical than horizontal
                                if velocity.y.abs() > velocity.x.abs() && velocity.y.abs() > 0.0 {
                                    // Vertical movement dominates, don't start the gesture
                                    gesture_state = SwipeBackGestureState::Cancelled;
                                } else {
                                    // Start the gesture
                                    gesture_state =
                                        SwipeBackGestureState::Swiping { distance: 0.0 };

                                    // Start a manual backward transition
                                    if self.current_transition.is_none() {
                                        let mut transition = CurrentTransition {
                                            active_transition: ActiveTransition::manual(
                                                self.backward_transition.clone(),
                                            )
                                            .with_default_duration(self.default_duration),
                                            leaving_route: None,
                                        };
                                        // Initialize progress to 1.0 (fully showing current page)
                                        transition.active_transition.set_progress(1.0);
                                        self.current_transition = Some(transition);
                                    }
                                }
                            }
                        }
                    }
                    SwipeBackGestureState::Swiping { distance, .. } => {
                        // Cancel if velocity becomes too vertical before we've committed
                        if distance < 10.0
                            && velocity.y.abs() > velocity.x.abs()
                            && velocity.y.abs() > 0.0
                        {
                            // Vertical movement dominates, cancel the gesture
                            self.current_transition = None;
                            gesture_state = SwipeBackGestureState::Cancelled;
                        } else {
                            // Update the gesture distance (only positive horizontal movement)
                            let new_distance = (distance + delta.x).max(0.0);

                            gesture_state = SwipeBackGestureState::Swiping {
                                distance: new_distance,
                            };

                            if new_distance > 10.0 {
                                // Steal the drag in case a scroll area is also detecting it
                                ui.ctx().set_dragged_id(gesture_id);
                            }

                            // Update the transition progress
                            if let Some(transition) = &mut self.current_transition {
                                let screen_width = content_rect.width();
                                let progress = 1.0 - (new_distance / screen_width);
                                transition.active_transition.set_progress(progress);
                            }
                        }
                    }
                    SwipeBackGestureState::Cancelled => {
                        // Wait for release before allowing new gestures
                    }
                }
            }

            if any_released {
                if let SwipeBackGestureState::Swiping { distance } = gesture_state {
                    // Velocity threshold for flick gesture (pixels per second)
                    const FLICK_VELOCITY_THRESHOLD: f32 = 100.0;

                    let screen_width = content_rect.width();
                    let progress = distance / screen_width;

                    // Check if we've swiped far enough OR flicked fast enough to trigger back navigation
                    let should_navigate_back = progress >= self.swipe_back_threshold
                        || velocity.x >= FLICK_VELOCITY_THRESHOLD;

                    if should_navigate_back {
                        let popped = self.history.pop();
                        // Complete the back navigation
                        if let Some(transition) = &mut self.current_transition {
                            let progress = transition.active_transition.progress();
                            transition.active_transition =
                                ActiveTransition::backward(self.backward_transition.clone());
                            transition.active_transition.set_progress(1.0 - progress);
                            transition.leaving_route = popped;
                        }
                    } else {
                        // Cancel the gesture - animate back to the current page
                        self.current_transition = None;
                    }

                    gesture_state = SwipeBackGestureState::Idle;
                } else {
                    gesture_state = SwipeBackGestureState::Idle;
                }
            }
        } else {
            // Pointer left the area, cancel the gesture
            if matches!(gesture_state, SwipeBackGestureState::Swiping { .. }) {
                self.current_transition = None;
            }
            gesture_state = SwipeBackGestureState::Idle;
        }

        // Save the gesture state
        ui.data_mut(|data| {
            data.insert_temp(gesture_id, gesture_state);
        });
    }
}
