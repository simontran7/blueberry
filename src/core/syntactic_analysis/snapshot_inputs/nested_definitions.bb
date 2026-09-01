func outer() {
    const LOCAL_THRESHOLD: I32 = 5;

    func helper(x: I32) -> I32 {
        x + LOCAL_THRESHOLD
    }

    helper(1);
}
