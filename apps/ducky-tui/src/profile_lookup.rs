use ducky_core::Profile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileLookupError {
    NotFound,
    Ambiguous {
        matching_profile_indexes: Vec<usize>,
    },
}

pub fn resolve_profile(profiles: &[Profile], query: &str) -> Result<usize, ProfileLookupError> {
    let matching_profile_indexes: Vec<usize> = profiles
        .iter()
        .enumerate()
        .filter(|(_, profile)| profile_matches(profile, query))
        .map(|(profile_index, _)| profile_index)
        .collect();
    match matching_profile_indexes.as_slice() {
        [] => Err(ProfileLookupError::NotFound),
        [profile_index] => Ok(*profile_index),
        _ => Err(ProfileLookupError::Ambiguous {
            matching_profile_indexes,
        }),
    }
}

fn profile_matches(profile: &Profile, query: &str) -> bool {
    if !profile.label.is_empty() && profile.label == query {
        return true;
    }
    let Some((project_name, rest)) = query.split_once('/') else {
        return false;
    };
    if profile.project.as_deref() != Some(project_name) {
        return false;
    }
    (!profile.label.is_empty() && profile.label == rest) || profile.kind.word() == rest
}

#[cfg(test)]
mod tests;
