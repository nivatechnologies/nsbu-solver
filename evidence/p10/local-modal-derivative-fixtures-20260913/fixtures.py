"""Exact integer oracle for local proposals of 39 Fourier derivative fixtures.

No trajectory, FFT, PDE acceptance, or reference field is evaluated here.
The fixture convention is a torus of side 2*pi, with derivative multiplier i*k.
"""
from __future__ import annotations


def cross(k: list[int], b: list[list[int]]) -> list[list[int]]:
    return [[k[(c + 1) % 3] * b[(c + 2) % 3][part]
             - k[(c + 2) % 3] * b[(c + 1) % 3][part]
             for part in range(2)] for c in range(3)]


def make_cases() -> list[dict]:
    modes = [[0, 0, 0], [1, 0, 0], [0, 2, 0], [0, 0, 3],
             [1, 2, 3], [-2, 1, 3], [-3, -1, 2], [2, 2, 1]]
    cases = []
    for index, k in enumerate(modes):
        left = cross(k, [[1, 1], [2, -1], [3, 2]])
        right = cross(k, [[-1, 2], [1, 3], [2, -2]])
        if index == 0:
            left, right = [[1, 0], [2, 0], [3, 0]], [[4, 0], [-1, 0], [0, 0]]
        cases.append({'wavevector': k, 'left': left, 'right': right,
                      'domain': '[0,2*pi)^3', 'case_id': f'mode-{index}'})
    return cases


def expected_rows(inputs: dict) -> list[list[int]]:
    k = inputs['wavevector']
    delta = [[r - l for r, l in zip(right, left)]
             for right, left in zip(inputs['right'], inputs['left'])]
    indices = [(c, ()) for c in range(3)]
    indices += [(c, (a,)) for c in range(3) for a in range(3)]
    indices += [(c, (a, b)) for c in range(3) for a in range(3) for b in range(3)]
    rows = []
    for component, axes in indices:
        re, im = delta[component]
        orders = [0, 0, 0]
        for axis in axes:
            orders[axis] += 1
            re, im = -k[axis] * im, k[axis] * re
        rows.append([component, *orders, re, im])
    return rows


def validate(output: dict, inputs: dict) -> list[str]:
    if set(output) != {'entries', 'scope'} or output.get('scope') != 'modal_derivative_fixture_only':
        return ['Require exactly entries and scope=modal_derivative_fixture_only.']
    rows = output.get('entries')
    if not isinstance(rows, list) or len(rows) != 39:
        return ['entries must contain exactly 39 ordered rows.']
    errors = []
    for index, (row, expected) in enumerate(zip(rows, expected_rows(inputs))):
        if not isinstance(row, list) or len(row) != 6 or any(type(x) is not int for x in row):
            errors.append(f'Row {index} must be six integers (booleans are invalid).')
        elif row != expected:
            errors.append(f'Row {index}: expected {expected}, got {row}. Check right-minus-left and i*k signs.')
    return errors[:4]


INSTRUCTIONS = '''Construct an exact small modal fixture for the regional diagnostic's
3 velocity, 9 gradient and 27 ordered Hessian components. The supplied left and
right entries are complex Fourier coefficients [real,imaginary]. Compute RIGHT
MINUS LEFT first. On the [0,2*pi)^3 torus, derivative along axis a multiplies a
coefficient by i*wavevector[a]. All arithmetic here is exact integer arithmetic.
Return exactly {"scope":"modal_derivative_fixture_only","entries":[...]}.
Each row is [component,dx,dy,dz,real,imaginary]. First output the 3 velocity rows
for component 0,1,2. Then the 9 gradient rows in component-major, derivative-axis
order. Finally output 27 Hessian rows in component-major, first-axis-major,
second-axis order. Both mixed derivative orders MUST appear as separate rows;
do not deduplicate them. There must be exactly 39 rows. No prose or markdown.
These are artificial fixtures, not PDE states or accepted-window evidence.'''


def self_check() -> None:
    for original in make_cases():
        for coefficients in (original['left'], original['right']):
            assert all(sum(k * a[part] for k, a in zip(original['wavevector'], coefficients)) == 0
                       for part in range(2))
        mirror = {**original, 'wavevector': [-k for k in original['wavevector']],
                  'left': [[r, -i] for r, i in original['left']],
                  'right': [[r, -i] for r, i in original['right']]}
        for direct, reflected in zip(expected_rows(original), expected_rows(mirror)):
            assert reflected == [*direct[:5], -direct[5]]
    # Closed-form independently hand-checked values for the mixed mode.
    case = make_cases()[4]
    rows = expected_rows(case)
    assert rows[0] == [0, 0, 0, 0, 1, -20]
    assert rows[3] == [0, 1, 0, 0, 20, 1]
    assert rows[13] == [0, 1, 1, 0, -2, 40]
    assert rows[15] == rows[13]  # xy and yx kept as distinct ordered entries
    output = {'scope': 'modal_derivative_fixture_only', 'entries': rows}
    assert validate(output, case) == []
    bad = [r[:] for r in rows]
    bad[0][4] = -bad[0][4]
    assert validate({'scope': output['scope'], 'entries': bad}, case)
    bad[0][4] = True
    assert validate({'scope': output['scope'], 'entries': bad}, case)


if __name__ == '__main__':
    self_check()
    print('Integer oracle checks passed; eight fixture cases prepared.')
