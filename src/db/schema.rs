table! {
    users (id) {
        id -> Int8,
        username -> Varchar,
        password -> Text,
        creation_date -> Timestamptz,
    }
}
