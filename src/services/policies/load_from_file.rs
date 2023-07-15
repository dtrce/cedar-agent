use std::path::PathBuf;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use log::{error, info};

use rocket::fairing::{Fairing, Info, Kind};
use rocket::serde::json::Json;
use rocket::Rocket;
use rocket::Build;

use crate::services::policies::PolicyStore;
use crate::schemas::policies::Policy;
use crate::config;

pub struct InitPoliciesFairing;

pub(crate) async fn init(conf: &config::Config, policy_store: &Box<dyn PolicyStore>) {
    let file_path = conf.policy.clone().unwrap_or("".to_string());

    if file_path.is_empty() {
        return;
    }

    let policies_file_path = PathBuf::from(&file_path);
    let policies = match load_policies_from_file(policies_file_path).await {
        Ok(policies) => policies,
        Err(err) => {
            error!("Failed to load policies from file: {}", err);
            return;
        }
    };

    match policy_store.update_policies(policies.into_inner()).await {
        Ok(policies) => {
            info!("Successfully updated policies from file {}: {} policies", &file_path, policies.len());
        }
        Err(err) => {
            error!("Failed to update policies: {}", err);
            return;
        }
    };
}

async fn load_policies_from_file(path: PathBuf) -> Result<Json<Vec<Policy>>, Box<dyn Error>> {
    // check if file exists
    if !path.exists() {
        return Err("File does not exist".into());
    }

    // check if is a valid json file
    if path.extension().unwrap() != "json" {
        return Err("File is not a json file".into());
    }

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to open file: {}", err).into()),
    };

    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        return Err(format!("Failed to read file: {}", err).into());
    }

    let policies: Vec<Policy> = match rocket::serde::json::from_str(&contents) {
        Ok(policies) => policies,
        Err(err) => return Err(format!("Failed to deserialize JSON: {}", err).into()),
    };
    
    Ok(Json(policies))
}

#[async_trait::async_trait]
impl Fairing for InitPoliciesFairing {
    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
        let config = rocket.state::<config::Config>().unwrap();
        init(config, rocket.state::<Box<dyn PolicyStore>>().unwrap()).await;

        Ok(rocket)
    }

    fn info(&self) -> Info {
        Info {
            name: "Init Policies",
            kind: Kind::Ignite
        }
    }
}