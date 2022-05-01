table! {
    sessions (id) {
        id -> Int8,
        user_id -> Int8,
        session_id -> Bytea,
        created_at -> Timestamptz,
        modified_at -> Nullable<Timestamptz>,
        expires_at -> Timestamptz,
    }
}

table! {
    users (id) {
        id -> Int8,
        username -> Varchar,
        password -> Text,
        created_at -> Timestamptz,
    }
}

joinable!(sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    sessions,
    users,
);
