use html::HtmlStringReply;
use serde::Deserialize;
use std::convert::Infallible;
use std::ops::Deref;
use warp::Filter;

mod html;
mod user;
mod verify;

const RESET_PASSWORD_PATHNAME: &str = "/reset-password";
const CREATE_USER_PATHNAME: &str = "/create-user";

#[derive(Debug)]
enum ServerError {
    RenderError,
    BadRequest,
}

#[derive(Debug, Deserialize)]
struct ResetFormParams {
    requested_password: String,
}

#[derive(Debug, Deserialize)]
struct NewUserParams {
    requested_email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserParams {
    requested_name: String,
    requested_password: String,
}

impl warp::reject::Reject for ServerError {}

async fn reset_password_post_handler(
    db: user::UserDatabase,
    url_params: verify::ResetParams,
    form_params: ResetFormParams,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    db.lock()
        .await
        .get_mut(&url_params.user_id())
        .ok_or_else(warp::reject::not_found)
        .and_then(|user| {
            let is_valid = verify::ResetParams::verify(user, &url_params);
            if is_valid {
                user.reset_password(&form_params.requested_password);
            }
            html::ResetPasswordTemplate::from_user_with_warning(user, is_valid)
                .as_html()
                .map(warp::reply::html)
                .map_err(|_| warp::reject::custom(ServerError::RenderError))
        })
}

async fn reset_password_get_handler(
    db: user::UserDatabase,
    params: verify::ResetParams,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    db.lock()
        .await
        .get(&params.user_id())
        .ok_or_else(warp::reject::not_found)
        .and_then(|user| {
            html::ResetPasswordTemplate::from_user(user)
                .as_html()
                .map(warp::reply::html)
                .map_err(|_| warp::reject::custom(ServerError::RenderError))
        })
}

async fn generate_reset_password_handler(
    id: user::UserId,
    db: user::UserDatabase,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    db.lock()
        .await
        .get(&id)
        .ok_or_else(warp::reject::not_found)
        .and_then(|user| {
            let params = verify::ResetParams::from(user);
            let url = html::create_url(RESET_PASSWORD_PATHNAME, Some(&params));
            html::GeneratePasswordResetTemplate::from_user_reset_link(user, &url)
                .as_html()
                .map(warp::reply::html)
                .map_err(|_| warp::reject::custom(ServerError::RenderError))
        })
}

async fn new_user_get_handler() -> Result<impl warp::Reply, warp::reject::Rejection> {
    html::NewUserTemplate::from_email(None)
        .as_html()
        .map(warp::reply::html)
        .map_err(|_| warp::reject::custom(ServerError::RenderError))
}

async fn new_user_post_handler(
    form_params: NewUserParams,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let email = form_params.requested_email.as_ref();
    let verify_params = verify::CreateParams::from(email);
    let url = html::create_url(CREATE_USER_PATHNAME, Some(&verify_params));
    let info = (url.as_ref(), email);
    html::NewUserTemplate::from_email(Some(info))
        .as_html()
        .map(warp::reply::html)
        .map_err(|_| warp::reject::custom(ServerError::RenderError))
}

async fn create_user_get_handler() -> Result<impl warp::Reply, warp::reject::Rejection> {
    html::CreateUserTemplate::form()
        .as_html()
        .map(warp::reply::html)
        .map_err(|_| warp::reject::custom(ServerError::RenderError))
}

async fn create_user_post_handler(
    db: user::UserDatabase,
    url_params: verify::CreateParams,
    form_params: CreateUserParams,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let requested_email = url_params.email();
    let CreateUserParams {
        requested_name,
        requested_password,
    } = form_params;
    let is_valid = verify::CreateParams::verify(requested_email, &url_params);

    if is_valid {
        let mut new_user = user::UserBuilder::new();
        new_user
            .with_email(&requested_email)
            .with_password(&requested_password)
            .with_name(&requested_name);
        db.add_user(new_user)
            .await
            .map_err(|_| warp::reject::custom(ServerError::BadRequest))?;
    }
    html::CreateUserTemplate::report_success(is_valid)
        .as_html()
        .map(warp::reply::html)
        .map_err(|_| warp::reject::custom(ServerError::RenderError))
}

async fn list_handler(db: user::UserDatabase) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let users = db.lock().await;
    html::ListUsersTemplate::from(users.deref())
        .as_html()
        .map(warp::reply::html)
        .map_err(|_| warp::reject::custom(ServerError::RenderError))
}

async fn rejection_handler(err: warp::reject::Rejection) -> Result<impl warp::Reply, Infallible> {
    let reply = warp::reply();
    let status_moded_reply = match err.find::<ServerError>() {
        Some(ServerError::BadRequest) => {
            warp::reply::with_status(reply, warp::http::StatusCode::BAD_REQUEST)
        }
        Some(ServerError::RenderError) => {
            warp::reply::with_status(reply, warp::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
        None => warp::reply::with_status(reply, warp::http::StatusCode::NOT_FOUND),
    };
    Ok(status_moded_reply)
}

#[tokio::main]
async fn main() {
    let user_db = user::UserDatabase::create_test_db();

    let list = warp::path("list")
        .and(warp::path::end())
        .and(user_db.inject())
        .and_then(list_handler);
    let reset_password_generate = warp::path("reset-password-generate")
        .and(warp::path::param())
        .and(warp::path::end())
        .and(user_db.inject())
        .and_then(generate_reset_password_handler);
    let reset_password_get = warp::path(&RESET_PASSWORD_PATHNAME[1..])
        .and(warp::path::end())
        .and(user_db.inject())
        .and(warp::query::<verify::ResetParams>())
        .and_then(reset_password_get_handler);
    let new_user_get = warp::path("new-user")
        .and(warp::path::end())
        .and_then(new_user_get_handler);
    let create_user_get = warp::path(&CREATE_USER_PATHNAME[1..])
        .and(warp::path::end())
        .and_then(create_user_get_handler);

    let get_routes = warp::get().and(
        list.or(reset_password_generate)
            .or(reset_password_get)
            .or(new_user_get)
            .or(create_user_get),
    );

    let reset_password_post = warp::path(&RESET_PASSWORD_PATHNAME[1..])
        .and(warp::path::end())
        .and(user_db.inject())
        .and(warp::query::<verify::ResetParams>())
        .and(warp::body::form::<ResetFormParams>())
        .and_then(reset_password_post_handler);
    let new_user_post = warp::path("new-user")
        .and(warp::path::end())
        .and(warp::body::form::<NewUserParams>())
        .and_then(new_user_post_handler);
    let create_user_post = warp::path(&CREATE_USER_PATHNAME[1..])
        .and(warp::path::end())
        .and(user_db.inject())
        .and(warp::query::<verify::CreateParams>())
        .and(warp::body::form::<CreateUserParams>())
        .and_then(create_user_post_handler);

    let post_routes = warp::post().and(reset_password_post.or(new_user_post).or(create_user_post));

    let routes = get_routes.or(post_routes).recover(rejection_handler);

    warp::serve(routes).run(([127, 0, 0, 1], 3232)).await;
}
