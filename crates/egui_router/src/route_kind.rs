use crate::{Route, RouteArg};

pub(crate) enum RouteKind<State> {
    Route(RouteHandler<State>),
    Redirect(String, RouteArg),
}

/// Route handler type for [`RouteKind`]
type RouteHandler<State> = Box<dyn FnMut() -> Box<dyn Route<State>>>;
