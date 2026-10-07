func tail() -> I32 {
    let a = { 1 };
    { a }
}

func no_tail() {
    let a = { 1; };
    {}
}

func statement_without_semicolon() -> Bool {
    if true { () } else { () }
    false
}
