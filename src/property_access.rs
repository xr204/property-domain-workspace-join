use crate::infrai_client::{InfraiClient, InfraiError};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaintenanceRequest {
    pub workspace: String,
    pub unit: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantDocument {
    pub workspace: String,
    pub tenant_email: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspectionReminder {
    pub workspace: String,
    pub property: String,
    pub due_on: String,
}

#[derive(Debug, Deserialize)]
struct DomainData {
    domain: String,
    zone_id: String,
    #[serde(default)]
    verified: bool,
}

#[derive(Debug, Deserialize)]
struct UserData {
    id: String,
    email: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum JoinDecision {
    Join { workspace: String },
    RejectDomainMismatch,
    AwaitDomainVerification,
}

#[derive(Debug, Serialize)]
pub struct JoinResult {
    pub user_id: String,
    pub email: String,
    pub workspace: String,
    pub created: bool,
}

#[derive(Debug, Error)]
pub enum JoinError {
    #[error(transparent)]
    Infrai(#[from] InfraiError),
    #[error("email must contain a company domain")]
    InvalidEmail,
    #[error("employee email does not match {0}")]
    DomainMismatch(String),
    #[error("{0} has not completed domain verification")]
    DomainUnverified(String),
}

pub fn decide_join(email: &str, domain: &str, verified: bool) -> JoinDecision {
    let email_domain = email.rsplit_once('@').map(|(_, value)| value);
    if email_domain != Some(domain) {
        JoinDecision::RejectDomainMismatch
    } else if !verified {
        JoinDecision::AwaitDomainVerification
    } else {
        JoinDecision::Join {
            workspace: domain.to_owned(),
        }
    }
}

pub async fn prove_domain(
    client: &InfraiClient,
    domain: &str,
    proof_name: &str,
    proof_content: &str,
) -> Result<String, JoinError> {
    let added: DomainData = client
        .write(
            Method::POST,
            "/v1/dns/domain/add",
            &json!({"domain": domain, "metadata": {"purpose": "property-workspace"}}),
        )
        .await?;

    let _: Value = client
        .write(
            Method::PUT,
            "/v1/dns/record/upsert",
            &json!({
                "zone_id": added.zone_id,
                "record_type": "TXT",
                "name": proof_name,
                "content": proof_content,
                "ttl": 300
            }),
        )
        .await?;
    Ok(added.zone_id)
}

pub async fn join_employee(
    client: &InfraiClient,
    domain: &str,
    email: &str,
    name: &str,
) -> Result<JoinResult, JoinError> {
    if !email.contains('@') {
        return Err(JoinError::InvalidEmail);
    }

    let verified: DomainData = client
        .write(
            Method::POST,
            "/v1/dns/domain/verify",
            &json!({"domain": domain}),
        )
        .await?;
    let workspace = match decide_join(email, &verified.domain, verified.verified) {
        JoinDecision::Join { workspace } => workspace,
        JoinDecision::RejectDomainMismatch => {
            return Err(JoinError::DomainMismatch(verified.domain));
        }
        JoinDecision::AwaitDomainVerification => {
            return Err(JoinError::DomainUnverified(verified.domain));
        }
    };

    match client
        .get::<UserData>("/v1/auth/user/get_by_email", &[("email", email)])
        .await
    {
        Ok(user) => Ok(JoinResult {
            user_id: user.id,
            email: user.email,
            workspace,
            created: false,
        }),
        Err(InfraiError::Api { status: 404, .. }) => {
            let user: UserData = client
                .write(
                    Method::POST,
                    "/v1/auth/user/create",
                    &json!({
                        "email": email,
                        "name": name,
                        "metadata": {"workspace": workspace, "role": "property_employee"},
                        "idempotency_key": format!("property-employee:{email}")
                    }),
                )
                .await?;
            Ok(JoinResult {
                user_id: user.id,
                email: user.email,
                workspace,
                created: true,
            })
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{decide_join, JoinDecision};

    #[test]
    fn joins_only_a_verified_employee_domain() {
        assert_eq!(
            decide_join("maya@harbor.example", "harbor.example", true),
            JoinDecision::Join {
                workspace: "harbor.example".to_owned()
            }
        );
        assert_eq!(
            decide_join("maya@contractor.example", "harbor.example", true),
            JoinDecision::RejectDomainMismatch
        );
        assert_eq!(
            decide_join("maya@harbor.example", "harbor.example", false),
            JoinDecision::AwaitDomainVerification
        );
    }
}
