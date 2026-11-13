#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Local,
    Staging,
    Prod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub label: String,
    pub url: String,
    pub kind: ProfileKind,
    pub project: Option<String>,
    pub tags: Vec<String>,
}

impl ProfileKind {
    pub fn word(self) -> &'static str {
        match self {
            ProfileKind::Local => "local",
            ProfileKind::Staging => "staging",
            ProfileKind::Prod => "prod",
        }
    }
}

impl Profile {
    pub fn display_name(&self) -> String {
        match (self.label.is_empty(), self.project.as_deref()) {
            (true, Some(project_name)) => format!("{project_name} {}", self.kind.word()),
            _ => self.label.clone(),
        }
    }

    pub fn target(&self) -> String {
        let without_scheme = self
            .url
            .split_once("://")
            .map_or(self.url.as_str(), |(_, rest)| rest);
        let without_query = without_scheme
            .split_once('?')
            .map_or(without_scheme, |(address, _)| address);
        let without_user_info = without_query
            .rsplit_once('@')
            .map_or(without_query, |(_, address)| address);
        without_user_info.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_with_url(url: &str) -> Profile {
        Profile {
            label: "A".to_owned(),
            url: url.to_owned(),
            kind: ProfileKind::Local,
            project: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn target_keeps_host_port_and_database() {
        let cases = [
            ("postgres://localhost/ducky", "localhost/ducky"),
            (
                "postgres://ducky@localhost:5440/ducky",
                "localhost:5440/ducky",
            ),
            (
                "postgresql://app:secret@db.example.com:5432/app",
                "db.example.com:5432/app",
            ),
            (
                "postgres://app:p@ss@db.example.com/app?sslmode=require",
                "db.example.com/app",
            ),
            (
                "postgres://app:${PROD_PW}@${DB_HOST}:5432/app",
                "${DB_HOST}:5432/app",
            ),
        ];
        for (url, expected_target) in cases {
            assert_eq!(profile_with_url(url).target(), expected_target, "{url}");
        }
    }

    #[test]
    fn display_name_prefers_label_then_project_and_kind() {
        let mut labelled_profile = profile_with_url("postgres://h/db");
        labelled_profile.project = Some("shop".to_owned());
        assert_eq!(labelled_profile.display_name(), "A");
        let unlabelled_profile = Profile {
            label: String::new(),
            kind: ProfileKind::Staging,
            ..labelled_profile
        };
        assert_eq!(unlabelled_profile.display_name(), "shop staging");
        let prod_profile = Profile {
            kind: ProfileKind::Prod,
            ..unlabelled_profile
        };
        assert_eq!(prod_profile.display_name(), "shop prod");
    }
}
