use crate::user::{User, UserTable};
use askama::Template;

pub trait HtmlStringReply {
    fn as_html(&self) -> Result<String, askama::Error>;
}

pub fn create_url(pathname: &str, qwargs: Option<&impl serde::Serialize>) -> String {
    match qwargs {
        Some(params) => {
            let qwargs = serde_url_params::to_string(params).unwrap();
            format!("{}?{}", pathname, qwargs)
        }
        None => pathname.to_string(),
    }
}

impl<T: Template> HtmlStringReply for T {
    fn as_html(&self) -> Result<String, askama::Error> {
        self.render()
    }
}

#[derive(Template)]
#[template(path = "generate_reset.html")]
pub struct GeneratePasswordResetTemplate<'a, 'b> {
    user: &'a User,
    link: &'b str,
}

impl<'a, 'b> GeneratePasswordResetTemplate<'a, 'b> {
    pub fn from_user_reset_link(user: &'a User, link: &'b str) -> Self {
        GeneratePasswordResetTemplate { user, link }
    }
}

#[derive(Template)]
#[template(path = "list.html")]
pub struct ListUsersTemplate<'a> {
    users: Vec<&'a User>,
}

impl<'a> From<&'a UserTable> for ListUsersTemplate<'a> {
    fn from(table: &'a UserTable) -> Self {
        let mut users = table.values().collect::<Vec<_>>();
        users.sort_unstable_by_key(|user| user.id);
        ListUsersTemplate { users }
    }
}

#[derive(Template)]
#[template(path = "reset_password.html")]
pub struct ResetPasswordTemplate<'a> {
    user: &'a User,
    success: Option<bool>,
}

impl<'a> ResetPasswordTemplate<'a> {
    pub fn from_user_with_warning(user: &'a User, is_valid: bool) -> Self {
        ResetPasswordTemplate {
            user,
            success: Some(is_valid),
        }
    }

    pub fn from_user(user: &'a User) -> Self {
        ResetPasswordTemplate {
            user,
            success: None,
        }
    }
}

#[derive(Template)]
#[template(path = "new_user.html")]
pub struct NewUserTemplate<'a> {
    email_info: Option<(&'a str, &'a str)>,
}

impl<'a> NewUserTemplate<'a> {
    pub fn from_email(email_info: Option<(&'a str, &'a str)>) -> Self {
        NewUserTemplate { email_info }
    }
}

#[derive(Template)]
#[template(path = "create_user.html")]
pub struct CreateUserTemplate {
    success: Option<bool>,
}

impl CreateUserTemplate {
    pub fn form() -> Self {
        CreateUserTemplate { success: None }
    }

    pub fn report_success(success: bool) -> Self {
        CreateUserTemplate {
            success: Some(success),
        }
    }
}
