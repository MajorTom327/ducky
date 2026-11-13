use ducky_core::RelationRef;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::Token;

use crate::tokens::SqlToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableRef {
    pub(crate) alias: Option<String>,
    pub(crate) relation: RelationRef,
}

pub(crate) fn table_refs(tokens: &[SqlToken], default_schema: &str) -> Vec<TableRef> {
    let significant_tokens: Vec<&Token> = tokens
        .iter()
        .map(|sql_token| &sql_token.token)
        .filter(|token| !matches!(token, Token::Whitespace(_)))
        .collect();
    let mut collected_refs = Vec::new();
    let mut token_index = 0;
    while token_index < significant_tokens.len() {
        let introducing_keyword = keyword_of(significant_tokens.get(token_index).copied());
        token_index += 1;
        if !matches!(
            introducing_keyword,
            Some(Keyword::FROM | Keyword::JOIN | Keyword::UPDATE | Keyword::INTO)
        ) {
            continue;
        }
        while let Some((table_ref, next_index)) =
            parse_table_ref(&significant_tokens, token_index, default_schema)
        {
            collected_refs.push(table_ref);
            token_index = next_index;
            let continues_from_list = introducing_keyword == Some(Keyword::FROM)
                && matches!(significant_tokens.get(token_index), Some(Token::Comma));
            if !continues_from_list {
                break;
            }
            token_index += 1;
        }
    }
    collected_refs
}

fn parse_table_ref(
    tokens: &[&Token],
    start_index: usize,
    default_schema: &str,
) -> Option<(TableRef, usize)> {
    let first_name = identifier_of(tokens.get(start_index).copied())?;
    let mut token_index = start_index + 1;
    let qualified_name = match tokens.get(token_index) {
        Some(Token::Period) => identifier_of(tokens.get(token_index + 1).copied()),
        _ => None,
    };
    let relation = match qualified_name {
        Some(relation_name) => {
            token_index += 2;
            RelationRef {
                schema: first_name,
                name: relation_name,
            }
        }
        None => RelationRef {
            schema: default_schema.to_owned(),
            name: first_name,
        },
    };
    let alias = if keyword_of(tokens.get(token_index).copied()) == Some(Keyword::AS) {
        token_index += 1;
        identifier_of(tokens.get(token_index).copied())
    } else {
        match tokens.get(token_index) {
            Some(Token::Word(word))
                if word.quote_style.is_some() || word.keyword == Keyword::NoKeyword =>
            {
                identifier_of(tokens.get(token_index).copied())
            }
            _ => None,
        }
    };
    if alias.is_some() {
        token_index += 1;
    }
    Some((TableRef { alias, relation }, token_index))
}

fn keyword_of(token: Option<&Token>) -> Option<Keyword> {
    match token {
        Some(Token::Word(word)) if word.quote_style.is_none() => Some(word.keyword),
        _ => None,
    }
}

fn identifier_of(token: Option<&Token>) -> Option<String> {
    match token {
        Some(Token::Word(word)) if word.quote_style.is_some() => Some(word.value.clone()),
        Some(Token::Word(word)) => Some(word.value.to_lowercase()),
        _ => None,
    }
}
