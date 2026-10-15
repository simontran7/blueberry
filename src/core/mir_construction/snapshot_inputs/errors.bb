func broken() -> I32 {
    true
}

func fine() -> I32 {
    1
}

func missing_initializer() -> I32 {
    let x: I32;
    x
}
