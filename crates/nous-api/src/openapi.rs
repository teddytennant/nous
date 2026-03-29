use utoipa::OpenApi;

use crate::files;
use crate::routes;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Nous API",
        version = "0.1.0",
        description = "Decentralized everything-app API — identity, social, governance, messaging, files, payments",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(name = "Teddy Tennant", email = "teddytennant@icloud.com")
    ),
    paths(
        routes::health,
        routes::node_info,
        routes::get_feed,
        routes::create_post,
        routes::get_event,
        routes::delete_event,
        routes::follow_user,
        routes::unfollow_user,
        routes::get_timeline,
        files::list_files,
        files::upload_file,
        files::get_file,
        files::get_latest,
        files::get_history,
        files::delete_file,
        files::store_stats,
    ),
    components(schemas(
        routes::HealthResponse,
        routes::NodeInfo,
        routes::FeedQuery,
        routes::CreatePostRequest,
        routes::FollowRequest,
        routes::TimelineQuery,
        files::FileListQuery,
        files::UploadRequest,
        files::LatestQuery,
        files::HistoryQuery,
        files::DeleteQuery,
        files::DeleteResponse,
    )),
    tags(
        (name = "node", description = "Node status and health"),
        (name = "social", description = "Social feed, posts, follows, timeline"),
        (name = "files", description = "Decentralized file storage with versioning and deduplication"),
    )
)]
pub struct NousApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_spec_generates() {
        let spec = NousApiDoc::openapi();
        let json = spec.to_json().unwrap();
        assert!(json.contains("Nous API"));
        assert!(json.contains("/api/v1/health"));
        assert!(json.contains("/api/v1/files"));
    }

    #[test]
    fn openapi_spec_has_correct_version() {
        let spec = NousApiDoc::openapi();
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["info"]["version"], "0.1.0");
    }

    #[test]
    fn openapi_spec_has_all_tags() {
        let spec = NousApiDoc::openapi();
        let json = serde_json::to_value(&spec).unwrap();
        let tags = json["tags"].as_array().unwrap();
        let tag_names: Vec<&str> = tags.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tag_names.contains(&"node"));
        assert!(tag_names.contains(&"social"));
        assert!(tag_names.contains(&"files"));
    }

    #[test]
    fn openapi_spec_has_schemas() {
        let spec = NousApiDoc::openapi();
        let json = serde_json::to_value(&spec).unwrap();
        let schemas = json["components"]["schemas"].as_object().unwrap();
        assert!(schemas.contains_key("HealthResponse"));
        assert!(schemas.contains_key("UploadRequest"));
        assert!(schemas.contains_key("DeleteResponse"));
    }
}
