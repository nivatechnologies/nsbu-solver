from analyze_timing import elapsed_seconds

assert elapsed_seconds("0:39.57") == 39.57
assert elapsed_seconds("1:02:03.45") == 3723.45
assert elapsed_seconds("12.34") == 12.34
