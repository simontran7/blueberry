func outer() -> I32 {
    const LOCAL: I32 = 5;

    func helper(x: I32) -> I32 {
        x + LOCAL
    }

    helper(1)
}
