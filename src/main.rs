use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::{get_service, post};
use axum::{routing::get, Router};
use chrono::{prelude::*, Duration};
use indexmap::IndexMap;
use maud::{html, Markup};
use nanoid::nanoid;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use tower_http::services::ServeDir;

// max number of db entries
static MAX_LEN: usize = 10000;

struct Entry {
    content: String,
    expiry: DateTime<Utc>,
}

#[derive(Deserialize)]
struct FormData {
    text: String,
}

// header helper function
fn header() -> Markup {
    html! {
        link rel="stylesheet" href="/static/styles.css";
        meta name="viewport" content="width=device-width";
        header {
            h1 {a href="/" {"zbin"}}
            nav {
                ul {a href="/about" {"about"}}
            }
        }
    }
}

async fn root() -> Markup {
    html! {
        (header())
        h1 {"new post"}
             form method="POST" action=("/") {
                            textarea name="text" {};
                            br{}
                            input type="submit" value=("submit");
                        }
    }
}

async fn view(
    Path(id): Path<String>,
    State(state): State<Arc<RwLock<IndexMap<String, Entry>>>>,
) -> Markup {
    let mut state = state.write().unwrap();
    let entry = state.get(&id);

    if entry.is_none() || Utc::now() > entry.unwrap().expiry {
        state.remove(&id);
        return html! {
            (header())
            p{"not found"}
        };
    }

    html! {
        (header())
        pre {
            (entry.unwrap().content)
        }
    }
}

// form helper function
async fn form(
    State(state): State<Arc<RwLock<IndexMap<String, Entry>>>>,
    Form(data): Form<FormData>,
) -> Redirect {
    let id = nanoid!();

    // ensure that only MAX_LEN elements are in map
    if state.read().unwrap().len() > MAX_LEN {
        state.write().unwrap().pop();
    }

    state.write().unwrap().insert(
        id.to_string(),
        Entry {
            content: data.text,
            expiry: Utc::now() + Duration::days(7),
        },
    );
    Redirect::to(&("/b/".to_owned() + &id))
}

async fn about() -> Markup {
    html! {(header())
        h1{"about this site"}
        p{"This site is a simple pastebin-style temporary text hosting site created by "{a href="https://blobco.de" {"blobcode"}}"."}
    }
}

#[tokio::main]
async fn main() {
    let mut entries: IndexMap<String, Entry> = IndexMap::new();
    // test entry
    entries.insert(
        "a".to_string(),
        Entry {
            content: "test".to_string(),
            expiry: Utc::now() + Duration::seconds(10),
        },
    );

    let state = Arc::new(RwLock::new(entries));
    // build our application with a single route
    let app = Router::new()
        .nest_service(
            "/static",
            get_service(ServeDir::new("./static")).handle_error(|error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("unhandled internal error {}", error),
                )
            }),
        )
        .route("/", get(root))
        .route("/b/:id", get(view))
        .route("/", post(form))
        .route("/about", get(about))
        .with_state(state);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
