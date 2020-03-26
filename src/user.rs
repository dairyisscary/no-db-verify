use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use warp::Filter;

pub type UserId = u64;

#[derive(Debug)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub bcrypt_password: String,
}

impl User {
    fn from(thread_rnd: &mut rand::rngs::ThreadRng, name: String) -> Self {
        let random_password = thread_rnd.gen::<u64>().to_string();
        let random_email = thread_rnd.gen::<u16>().to_string();
        User {
            id: thread_rnd.gen(),
            name,
            email: format!("user-{}@spookysoftware.dev", random_email),
            bcrypt_password: bcrypt::hash(&random_password, 4).unwrap(),
        }
    }

    pub fn reset_password(&mut self, new_password: &str) {
        self.bcrypt_password = bcrypt::hash(new_password, 4).unwrap();
    }
}

#[derive(Debug)]
pub struct UserBuilder {
    requested_name: Option<String>,
    requested_email: Option<String>,
    requested_password: Option<String>,
}

impl UserBuilder {
    pub fn new() -> Self {
        UserBuilder {
            requested_name: None,
            requested_email: None,
            requested_password: None,
        }
    }

    pub fn with_email(&mut self, email: &str) -> &mut Self {
        self.requested_email = Some(email.to_string());
        self
    }

    pub fn with_name(&mut self, name: &str) -> &mut Self {
        self.requested_name = Some(name.to_string());
        self
    }

    pub fn with_password(&mut self, password: &str) -> &mut Self {
        self.requested_password = Some(password.to_string());
        self
    }

    fn build(self) -> Option<User> {
        let name = self.requested_name?;
        let email = self.requested_email?;
        let password = self.requested_password?;
        let rnd = &mut rand::thread_rng();
        Some(User {
            id: rnd.gen(),
            name,
            email,
            bcrypt_password: bcrypt::hash(&password, 4).unwrap(),
        })
    }
}

pub type UserTable = HashMap<UserId, User>;

#[derive(Debug, Clone)]
pub struct UserDatabase {
    db: Arc<Mutex<UserTable>>,
}

impl UserDatabase {
    pub fn create_test_db() -> Self {
        let mut users = HashMap::new();
        let rnd = &mut rand::thread_rng();

        let user = User::from(rnd, "Eric".into());
        users.insert(user.id, user);

        let user = User::from(rnd, "Linus".into());
        users.insert(user.id, user);

        let user = User::from(rnd, "Michelle".into());
        users.insert(user.id, user);

        let user = User::from(rnd, "Rogan".into());
        users.insert(user.id, user);

        let user = User::from(rnd, "Lily".into());
        users.insert(user.id, user);

        let mut user = User::from(rnd, "Neo".into());
        user.id = 1;
        users.insert(1, user);

        let db = Arc::new(Mutex::new(users));
        UserDatabase { db }
    }

    pub fn inject(
        &self,
    ) -> impl Filter<Extract = (Self,), Error = std::convert::Infallible> + Clone {
        let hanging_copy = self.clone();
        warp::any().map(move || hanging_copy.clone())
    }

    pub async fn lock(&self) -> MutexGuard<'_, UserTable> {
        self.db.lock().await
    }

    pub async fn add_user(&self, built_user: UserBuilder) -> Result<(), ()> {
        let real_user = built_user.build().ok_or(())?;
        let mut users = self.lock().await;
        let duplicate = users.values().any(|user| user.email == real_user.email);
        if duplicate {
            Err(())
        } else {
            users.insert(real_user.id, real_user);
            Ok(())
        }
    }
}
