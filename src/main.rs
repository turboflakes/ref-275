use yew::prelude::*;
use yew_router::prelude::*;

use crate::vote::VoteComponent;
mod services;
mod vote;

#[derive(Routable, PartialEq, Eq, Clone, Debug)]
pub enum Route {
    // #[not_found]
    #[at("/")]
    Home,
}

fn main() {
    yew::Renderer::<App>::new().render();
}

struct App;

impl Component for App {
    type Message = ();

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        App
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                <Switch<Route> render={switch} />
            </BrowserRouter>
        }
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <VoteComponent/> },
    }
}
