use actix_web::{dev::Payload, FromRequest, HttpRequest};
use std::sync::{Arc, RwLock};
use uuidv7::Uuid;

use jsonwebtoken::dangerous_unsafe_decode;

use crate::{AppError, Connection, Pool};


#[derive(Deserialize, Debug, Default, Clone)]
pub struct AppContext {
    #[serde(skip)]
    pub db_pool: Option<Arc<Pool>>,

    pub email: String,

    #[serde(rename = "cognito:groups")]
    pub roles: Option<Vec<String>>,

    #[serde(rename = "custom:organizationId")]
    pub organization_id: String,

    #[serde(skip)]
    pub organization_uuid: Uuid,

    #[serde(rename = "custom:tenantId")]
    pub tenant_id: String,

    #[serde(skip)]
    pub tenant_uuid: Uuid,

    #[serde(rename = "custom:personId")]
    pub person_id: String,

    #[serde(skip)]
    pub person_uuid: Uuid,

    #[serde(skip)]
    pub app_id: Option<String>,

    #[serde(skip)]
    pub prefetch: Arc<RwLock<Prefetch>>
}

#[derive(Debug, Default)]
pub struct Prefetch{
    pub id_list_orgs: Vec<Uuid>,
    pub id_list_groups: Vec<Uuid>,
    pub id_list_people: Vec<Uuid>,
    pub id_list_contacts: Vec<Uuid>,
    

    pub organization: super::Organization,
    pub group: super::Group,
    pub person: super::Person,
    pub contact: super::Contact
}


impl AppContext {
    pub fn get_conn(&self) -> Connection {
        self.db_pool.clone().unwrap().get().unwrap()
    }
}


impl juniper::Context for AppContext {}

const KEY_JWT_AUTHORIZATION: &str = "Authorization";
const KEY_JWT_ASSERTION: &str = "x-jwt-assertion";

impl <'a> FromRequest for AppContext {
    type Error = AppError;
    type Future = Result<AppContext, AppError>;
    type Config = ();

    fn from_request(req: &HttpRequest, _pl: &mut Payload) -> Self::Future {
        let headers = req.headers();

        //info!("AUTH.0:{:?}", headers.get(KEY_JWT_AUTHORIZATION));
        //info!("ASSERTION:{:?}", headers.get(KEY_JWT_ASSERTION));

        let token = req
            .headers()
            .get(KEY_JWT_ASSERTION)
            .map(|value| value.to_str().ok())
            .ok_or_else(||AppError::Unauthorized("Invalid JWT".into()))?;

        match token {
            Some(t) => {
                // check that the token is valid - up to you how you do this
                match verify_token(&t) {
                    Ok(claims) => {
                        Ok(claims)
                    }
                    Err(_) => {
                        //e is a string message
                        Err(AppError::Unauthorized("Unable to validate JWT".into()))
                    }
                }
            }
            None => {
                Err(AppError::Unauthorized("Unable to parse JWT".into()))
            }
        }
    }
}

fn verify_token(token: &str) -> Result<AppContext, String> {
    if log_enabled!(log::Level::Trace) {
        debug!("parsing:{}", token);
    }

    let mut token = dangerous_unsafe_decode::<AppContext>(token)
        .map_err(|_| "Unable to parse key!".to_owned())?;

    //cache
    token.claims.organization_uuid =
        Uuid::parse_str(&token.claims.organization_id).map_err(|e| {
            format!(
                "Unable to parse organization_uuid:{}. Error:{}",
                token.claims.organization_id, e
            )
        })?;
    token.claims.tenant_uuid = Uuid::parse_str(&token.claims.tenant_id).map_err(|e| {
        format!(
            "Unable to parse tennant_uuid:{}. Error:{}",
            token.claims.tenant_id, e
        )
    })?;

    token.claims.person_uuid = Uuid::parse_str(&token.claims.person_id).map_err(|e| {
        format!(
            "Unable to parse person_uuid:{}. Error:{}",
            token.claims.person_id, e
        )
    })?;


    //debug!("claims:{:?}", token.claims);

    Ok(token.claims)
}
