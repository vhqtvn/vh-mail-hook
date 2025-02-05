use schemars::JsonSchema;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, JsonSchema)]
struct SwaggerSpec {
    swagger: String,
    info: Info,
    paths: HashMap<String, PathItem>,
    definitions: HashMap<String, Schema>,
    #[serde(rename = "securityDefinitions")]
    security_definitions: HashMap<String, SecurityScheme>,
}

#[derive(Serialize, JsonSchema)]
struct Info {
    title: String,
    version: String,
    description: String,
}

#[derive(Serialize, JsonSchema)]
struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete: Option<Operation>,
    parameters: Vec<Parameter>,
}

#[derive(Serialize, JsonSchema)]
struct Operation {
    summary: String,
    description: String,
    responses: HashMap<String, Response>,
    security: Vec<HashMap<String, Vec<String>>>,
}

#[derive(Serialize, JsonSchema)]
struct Parameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    description: String,
    required: bool,
    #[serde(rename = "type")]
    type_: String,
    format: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct Response {
    description: String,
    schema: Option<Schema>,
}

#[derive(Serialize, JsonSchema)]
struct Schema {
    #[serde(rename = "type")]
    type_: String,
    format: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct SecurityScheme {
    #[serde(rename = "type")]
    type_: String,
    description: String,
    name: String,
    #[serde(rename = "in")]
    location: String,
}

fn parse_doc_comment(doc: &str) -> (String, String, HashMap<String, String>) {
    let lines = doc.lines()
        .map(|line| line.trim_start_matches("///").trim())
        .collect::<Vec<_>>();

    // First non-empty line is summary
    let summary = lines.iter()
        .find(|line| !line.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Find description (text between summary and first section)
    let mut description = String::new();
    let mut i = 1;
    while i < lines.len() && !lines[i].contains(':') {
        if !lines[i].is_empty() {
            description.push_str(lines[i]);
            description.push('\n');
        }
        i += 1;
    }

    // Parse sections (Returns:, Parameters:, etc.)
    let mut sections = HashMap::new();
    let mut current_section = String::new();
    let mut current_content = String::new();

    while i < lines.len() {
        let line = lines[i].trim();
        if line.contains(':') && (line.starts_with("Returns") || line.starts_with("Parameters") || line.starts_with("Authorization")) {
            if !current_section.is_empty() {
                sections.insert(current_section.clone(), current_content.trim().to_string());
                current_content.clear();
            }
            current_section = line.split(':').next().unwrap_or_default().trim().to_string();
        } else if !line.is_empty() && !line.starts_with("```") {
            current_content.push_str(line);
            current_content.push('\n');
        }
        i += 1;
    }

    if !current_section.is_empty() {
        sections.insert(current_section, current_content.trim().to_string());
    }

    // Ensure we have a Returns section
    if !sections.contains_key("Returns") {
        sections.insert("Returns".to_string(), "200: Success".to_string());
    }

    (summary, description.trim().to_string(), sections)
}

fn parse_parameters(params: &str) -> Vec<Parameter> {
    params.lines()
        .filter(|line| line.starts_with('-'))
        .map(|line| {
            let parts: Vec<&str> = line.trim_start_matches('-').trim().split(':').collect();
            let name = parts[0].trim();
            let description = parts.get(1).map(|s| s.trim()).unwrap_or_default();

            let name = if name.starts_with('`') && name.ends_with('`') {
                &name[1..name.len() - 1]
            } else {
                name
            };
            
            Parameter {
                name: name.to_string(),
                location: "path".to_string(),
                description: description.to_string(),
                required: true,
                type_: "string".to_string(),
                format: None,
            }
        })
        .collect()
}

fn parse_responses(responses: &str) -> HashMap<String, Response> {
    let mut map = HashMap::new();
    
    for line in responses.lines() {
        if let Some(line) = line.trim().strip_prefix('-') {
            let parts: Vec<&str> = line.trim().split(':').collect();
            if parts.len() == 2 {
                let code = parts[0].trim();
                let description = parts[1].trim();
                
                let schema = if code == "200" {
                    Some(Schema {
                        type_: if description.contains("List") { "array" } else { "object" }.to_string(),
                        format: None,
                    })
                } else {
                    None
                };

                map.insert(
                    code.to_string(),
                    Response {
                        description: description.to_string(),
                        schema,
                    },
                );
            }
        }
    }
    
    map
}

pub fn generate_spec() -> String {
    let mut paths = HashMap::new();
    let definitions = HashMap::new();

    // Add security definitions
    let mut security_definitions = HashMap::new();
    security_definitions.insert(
        "apiKey".to_string(),
        SecurityScheme {
            type_: "apiKey".to_string(),
            description: "API Key for authentication".to_string(),
            name: "Authorization".to_string(),
            location: "header".to_string(),
        },
    );

    // Get doc comments from the source code
    let lib_contents = include_str!("lib.rs");

    // Extract doc comments by first finding the function, then getting the doc after @APIDOC-START
    let api_get_mailbox_emails_doc = lib_contents
        .split("async fn api_get_mailbox_emails")
        .next()
        .unwrap()
        .split("// @APIDOC-START")
        .last()
        .unwrap();

    let api_get_email_doc = lib_contents
        .split("async fn api_get_email")
        .next()
        .unwrap()
        .split("// @APIDOC-START")
        .last()
        .unwrap();

    let api_delete_email_doc = lib_contents
        .split("async fn api_delete_email")
        .next()
        .unwrap()
        .split("// @APIDOC-START")
        .last()
        .unwrap();

    // Parse doc comments and generate paths
    let (list_summary, list_desc, list_sections) = parse_doc_comment(api_get_mailbox_emails_doc);
    let (get_summary, get_desc, get_sections) = parse_doc_comment(api_get_email_doc);
    let (delete_summary, delete_desc, delete_sections) = parse_doc_comment(api_delete_email_doc);

    // Add list emails path
    paths.insert(
        "/api/v1/mailboxes/{id}/emails".to_string(),
        PathItem {
            get: Some(Operation {
                summary: list_summary,
                description: list_desc,
                responses: parse_responses(&list_sections["Returns"]),
                security: vec![{
                    let mut security = HashMap::new();
                    security.insert("apiKey".to_string(), vec![]);
                    security
                }],
            }),
            delete: None,
            parameters: parse_parameters(&list_sections["Parameters"]),
        },
    );

    // Add single email operations
    paths.insert(
        "/api/v1/mailboxes/{mailbox_id}/emails/{email_id}".to_string(),
        PathItem {
            get: Some(Operation {
                summary: get_summary,
                description: get_desc,
                responses: parse_responses(&get_sections["Returns"]),
                security: vec![{
                    let mut security = HashMap::new();
                    security.insert("apiKey".to_string(), vec![]);
                    security
                }],
            }),
            delete: Some(Operation {
                summary: delete_summary,
                description: delete_desc,
                responses: parse_responses(&delete_sections["Returns"]),
                security: vec![{
                    let mut security = HashMap::new();
                    security.insert("apiKey".to_string(), vec![]);
                    security
                }],
            }),
            parameters: parse_parameters(&get_sections["Parameters"]),
        },
    );

    let spec = SwaggerSpec {
        swagger: "2.0".to_string(),
        info: Info {
            title: "VH Mail Hook API".to_string(),
            version: "1.0.0".to_string(),
            description: "API for managing email hooks. For examples and usage guide, see: https://github.com/vhqtvn/vh-mail-hook/tree/main/examples".to_string(),
        },
        paths,
        definitions,
        security_definitions,
    };

    serde_json::to_string_pretty(&spec).unwrap()
}

// Create a static string containing the Swagger specification
lazy_static::lazy_static! {
    pub static ref SWAGGER_SPEC: String = generate_spec();
} 