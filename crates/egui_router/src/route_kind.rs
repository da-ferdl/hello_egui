use crate::{RouteArg, RouteHandler};

pub(crate) enum RouteKind<State> {
    Route(RouteHandler<State>),
    Redirect(String, RouteArg),
}
