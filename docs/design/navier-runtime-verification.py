"""Reproduce revision 0.8 checks of the adopted revision 0.7 runtime design.

This is not a Navier--Stokes solver.

Run: python3 navier-runtime-verification.py --output results.json
Requires Python 3 and SymPy. Uses Decimal for independent coefficient references.
"""

from __future__ import annotations

import argparse
from decimal import Decimal, ROUND_CEILING, localcontext
import json
import math
from pathlib import Path
from fractions import Fraction


def phi(z: Decimal, j: int) -> Decimal:
    if z == 0:
        return Decimal(1) / Decimal(math.factorial(j))
    numerator = z.exp() - sum(
        (z**m / Decimal(math.factorial(m)) for m in range(j)), Decimal(0)
    )
    return numerator / z**j


def weights(z: Decimal) -> tuple[Decimal, Decimal, Decimal]:
    p1, p2, p3 = (phi(z, j) for j in (1, 2, 3))
    return p1 - 3*p2 + 4*p3, 2*p2 - 4*p3, -p2 + 4*p3


def scale_checks() -> dict:
    hbar = (-Decimal(11)).exp()
    ln10 = Decimal(10).ln()
    counts = {
        "h_upper_relaxation_Md_0": str(hbar),
        "minus_log10_Q_at_epsilon_quarter": str(Decimal(4).ln()/hbar/ln10),
        "minus_log10_tau_at_normalized_growth_10": str(1/hbar),
        "first_dyadic_band_at_epsilon_quarter_relaxed_h": int(
            (2/hbar).to_integral_value(rounding=ROUND_CEILING)
        ),
        "log10_annulus_radius_ratio_lower_relaxation": str(
            (Decimal(3)+Decimal(110).ln()+110-Decimal(4).ln())/(2*ln10)
        ),
    }
    carriers = []
    for eps in map(Decimal, ("0.99", "0.5", "0.25", "0.249", "0.01")):
        carrier = int((1/eps.sqrt()).to_integral_value(rounding=ROUND_CEILING))
        carriers.append({"epsilon": str(eps), "carrier": carrier})
    assert [item["carrier"] for item in carriers] == [2, 2, 2, 3, 10]
    # A short significand can store an extremely small distance when exponent
    # storage is separate. Forming the difference 1-tau is a different operation.
    tiny = Decimal(2)**-120000
    assert tiny != 0 and float(tiny) == 0
    assert 1-tiny == 1  # This Decimal context, too, loses the subtraction.
    growth = (hbar * Decimal(-120000) * Decimal(2).ln()).exp()
    counts.update({"ceiling_examples": carriers,
                   "scaled_time_example": {"mantissa": 1, "binary_exponent": -120000,
                                            "Q_to_h": str(growth)},
                   "interpretation": "Relaxed parameter bounds, not an admissible source instance."})
    return counts


def coefficient_checks() -> dict:
    examples = []
    for zs in ("0", "-0.5", "-2", "-10", "-50", "-100", "-10000"):
        z = Decimal(zs)
        w1, w2, w3 = weights(z)
        assert abs(w1+2*w2+w3-phi(z, 1)) < Decimal("1e-65")
        if z <= -50:
            good = (-1/z**2-4/z**3, 2/z**2+4/z**3, -1/z-3/z**2-4/z**3)
            bad = (-1/z**2-4/z**3, 2/z**2+8/z**3, -1/(2*z)-3/z**2-4/z**3)
            corrected_errors = [float(abs((a-b)/b)) for a,b in zip(good,(w1,w2,w3))]
            assert max(corrected_errors) < 1e-18
            examples.append({"z": zs,
                             "corrected_asymptotic_relative_errors": corrected_errors,
                             "rev05_asymptotic_relative_errors": [float(abs((a-b)/b)) for a,b in zip(bad,(w1,w2,w3))]})
    return {"constant_source_identity": "w1 + 2*w2 + w3 = phi1 passed",
            "large_negative_examples": examples}


def source_screening_checks() -> dict:
    """Evaluate a sufficient proof screen, never an admitted source instance."""
    h = (-Decimal(11)).exp()
    ln2 = Decimal(2).ln()
    ln10 = Decimal(10).ln()
    quarter_band = int((2/h).to_integral_value(rounding=ROUND_CEILING))

    def row(ell: int) -> dict:
        eps = (-h*ell*ln2).exp()
        carrier = int((1/eps.sqrt()).to_integral_value(rounding=ROUND_CEILING))
        slow = ell**2
        test = Decimal(slow**2)*(eps + eps**2 + Decimal(1)/carrier)
        envelope = Decimal(slow**2)*(eps + eps**2 + eps.sqrt())
        return {"ell": ell, "epsilon": str(eps), "carrier": carrier,
                "S_star": slow, "S_star_cubed": str(slow**3),
                "S_star_ninth_power": str(slow**9),
                "chart_mesh_S_star_minus_3": str(Decimal(1)/slow**3),
                "explicit_proof_test": str(test),
                "smooth_sufficient_envelope": str(envelope),
                "minus_log10_Q": str(ell*ln2/ln10),
                "auxiliary_traversal_chart_proxy_epsilon_S": str(eps*slow),
                "auxiliary_gaussian_chart_proxy_epsilon_sqrtS": str(eps*ell)}

    # Each term of ell^4*(eps+eps^2+sqrt(eps)) decreases after this point.
    monotone_start = int((8/(h*ln2)).to_integral_value(rounding=ROUND_CEILING))
    lower = max(quarter_band, monotone_start)
    assert Decimal(row(lower)["smooth_sufficient_envelope"]) > 1
    upper = lower
    for _ in range(32):
        upper *= 2
        if Decimal(row(upper)["smooth_sufficient_envelope"]) <= 1:
            break
    else:
        raise ArithmeticError("No bracket for sufficient smallness screen")
    for _ in range(64):
        if upper-lower == 1:
            break
        middle = (lower+upper)//2
        if Decimal(row(middle)["smooth_sufficient_envelope"]) <= 1:
            upper = middle
        else:
            lower = middle
    assert upper-lower == 1
    assert Decimal(row(upper-1)["smooth_sufficient_envelope"]) > 1
    assert Decimal(row(upper)["smooth_sufficient_envelope"]) <= 1
    assert Decimal(row(quarter_band)["explicit_proof_test"]) > 1

    sqrt2 = Decimal(2).sqrt()
    lam, temporal = 4-sqrt2, 4+sqrt2
    rho = lam.ln()/temporal.ln()
    dr = 2*((1+h)*rho-h*Decimal("0.00001"))
    maps = []
    for ell in (quarter_band, upper):
        slow = Decimal(ell**2)
        covering = int(((1+h)*ell*ln2-slow.ln())//temporal.ln())
        ci = (covering*temporal.ln()-(1+h)*ell*ln2).exp()
        mi = (covering*lam.ln()-dr*ell*ln2/2).exp()
        assert 1/temporal < ci*slow <= 1
        maps.append({"ell": ell, "covering_index": covering,
                     "c_i": str(ci), "M_i": str(mi)})
    return {"h_is_relaxed_boundary_not_admissible": str(h),
            "quarter_screen": row(quarter_band),
            "sufficient_screen_predecessor": row(upper-1),
            "sufficient_screen_first_band_in_monotone_range": row(upper),
            "auxiliary_map": {"rho_g": str(rho), "d_r": str(dr),
                              "rows": maps},
            "q_star": None, "source_profiles_computed": False,
            "interpretation": "One displayed sufficient test from Lemma 7.1. Not a necessary condition for every realization; not the complete q_star gate. S powers are mesh scale/count factors, not measured pulse counts or universal work lower bounds."}


def source_mesh_review_checks(screen: dict) -> dict:
    """Check review arithmetic and a declared explicit-mesh resource policy.

    The policy deliberately resolves every slow-mesh interval on a prescribed
    chart segment. It is not a lower bound on every independent PDE method.
    Amplitude factors omit the source's logarithmic/profile/weight factors.
    """
    h = Decimal(screen["h_is_relaxed_boundary_not_admissible"])
    ln2 = Decimal(2).ln()
    illustrative_lambda = Decimal(128)
    radius = (8/illustrative_lambda).sqrt()
    segment = Decimal(1)/4
    max_intervals = 1024
    assert radius == segment
    rows = []
    for source in (screen["quarter_screen"],
                   screen["sufficient_screen_first_band_in_monotone_range"]):
        ell = source["ell"]
        eps = Decimal(source["epsilon"])
        carrier = source["carrier"]
        mesh_denominator = ell**6
        mesh = Decimal(1)/mesh_denominator
        required_intervals = (mesh_denominator+3)//4
        assert 4*required_intervals >= mesh_denominator
        assert 4*(required_intervals-1) < mesh_denominator
        assert mesh == Decimal(source["chart_mesh_S_star_minus_3"])
        assert eps.sqrt() <= Decimal(1)/2
        assert required_intervals > max_intervals
        traversal = eps*ell**2
        gaussian = eps*ell
        rows.append({
            "ell": ell,
            "sqrt_epsilon_pure_asymptotic_factor": str(eps.sqrt()),
            "chart_mesh_spacing": str(mesh),
            "slow_support_full_extent_upper_bound": str(2*mesh),
            "carrier_over_ell6": str(Decimal(carrier)/mesh_denominator),
            "mesh_over_illustrative_core_radius": str(mesh/radius),
            "auxiliary_traversal_chart_proxy": str(traversal),
            "auxiliary_gaussian_chart_proxy": str(gaussian),
            "auxiliary_traversal_proxy_over_slow_full_extent_bound": str(traversal/(2*mesh)),
            "auxiliary_gaussian_proxy_over_slow_full_extent_bound": str(gaussian/(2*mesh)),
            "policy_required_intervals_per_axis": str(required_intervals),
            "policy_exceeds_cap": True,
        })

    # Check the review's proposed carrier/mesh balance as a heuristic only.
    # Its logarithmic proxy is strictly increasing beyond 12/(h log 2).
    def balance(ell: int) -> Decimal:
        return h*ell*ln2/2-6*Decimal(ell).ln()

    lower = int((12/(h*ln2)).to_integral_value(rounding=ROUND_CEILING))
    assert balance(lower) < 0
    upper = lower
    for _ in range(32):
        upper *= 2
        if balance(upper) >= 0:
            break
    else:
        raise ArithmeticError("No bracket for heuristic carrier/mesh balance")
    for _ in range(64):
        if upper-lower == 1:
            break
        middle = (lower+upper)//2
        if balance(middle) >= 0:
            upper = middle
        else:
            lower = middle
    assert upper-lower == 1 and balance(lower) < 0 <= balance(upper)
    crossings = []
    for ell in (lower, upper):
        carrier = int((h*ell*ln2/2).exp().to_integral_value(rounding=ROUND_CEILING))
        crossings.append({"ell": ell, "carrier": str(carrier),
                          "ell6": str(ell**6),
                          "carrier_over_ell6": str(Decimal(carrier)/ell**6)})
    assert int(crossings[0]["carrier"]) < lower**6
    assert int(crossings[1]["carrier"]) >= upper**6
    return {
        "status": "passed",
        "arithmetic_precision_decimal_digits": 75,
        "illustrative_core": {"Lambda": "128", "chart_radius": "1/4",
                              "source_admissibility_claimed": False},
        "rows": rows,
        "amplitude_scope": "Pure q^(h/2) at q=Q only; omitted constants, logarithms, profiles and support weights prevent an absolute pulse amplitude bound. No pulse omission is authorized.",
        "time_scope": "The full slow cutoff extent is at most 2 Q ell^-6 in physical time. Q^(1+h)L_s is an auxiliary-rectangle traversal duration. Actual support is their intersection with all other cutoffs; epsilon*S and epsilon*sqrt(S) are proxies with unknown constants.",
        "heuristic_carrier_mesh_crossing": {
            "classification": "Scale balance only; neither a necessary admissibility test nor the source q_star",
            "predecessor": crossings[0], "first_passing_in_monotone_range": crossings[1]},
        "explicit_mesh_policy": {
            "policy_id": "explicit-slow-mesh-v1",
            "chart_segment_length": "1/4",
            "required_spacing_at_most": "ell^-6",
            "maximum_stored_intervals_per_axis": max_intervals,
            "cap_origin": "Declared policy for this screen, not measured hardware capacity or a universal resource bound",
            "minimum_tested_band": screen["quarter_screen"]["ell"],
            "band_scope": "Both displayed relaxed bands and every larger integer under this same policy; required intervals increase as ell^6",
            "scope": "Explicit subdivision of every mesh interval across the declared chart segment, conditional on undertaking that mesh-resolving plan; actual localized active support has not been admitted",
            "status": "FeasibilityExcluded",
            "reason": "ceil((1/4)*ell^6) exceeds the declared 1024-interval cap in each tested row",
            "universal_representation_exclusion_claimed": False,
        },
        "literal_target_status": "FeasibilityUnestablished",
        "direct_reproduction_in_current_release": False,
        "compiler_scope": "MathematicsVerificationOnly",
        "averaged_stress_surrogate": "SeparateDecisionDeferred",
    }


def float_phi(z: float, j: int) -> float:
    if abs(z) <= 1:
        value = 1/math.factorial(18+j)
        for m in range(17, -1, -1):
            value = value*z + 1/math.factorial(m+j)
        return value
    value = math.expm1(z)/z
    for index in range(1, j):
        value = (value-1/math.factorial(index))/z
    return value


def float_weights(z: float) -> tuple[float, float, float]:
    if abs(z) <= 1:
        a, b, c = (float_phi(z, j) for j in (1, 2, 3))
        return a-3*b+4*c, 2*b-4*c, -b+4*c
    if z <= -50:
        r = 1/z
        return -r*r*(1+4*r), 2*r*r*(1+2*r), -r*(1+3*r+4*r*r)
    e = math.exp(z)
    return ((e*(z*z-3*z+4)-z-4)/z**3,
            2*(e*(z-2)+z+2)/z**3,
            (e*(4-z)-z*z-3*z-4)/z**3)


def floating_junction_checks() -> dict:
    rows = []
    values = [math.nextafter(-50.0, -math.inf), -50.0,
              math.nextafter(-50.0, math.inf),
              math.nextafter(-1.0, -math.inf), -1.0,
              math.nextafter(-1.0, math.inf)]
    for z in values:
        exact = weights(Decimal.from_float(z))
        observed = float_weights(z)
        errors = [float(abs(Decimal.from_float(a)-b)) for a, b in zip(observed, exact)]
        conditioning_scale = sum(abs(float(phi(Decimal.from_float(z), j))) for j in (1,2,3))
        assert max(errors) <= 256*math.ulp(1.0)*conditioning_scale
        for argument in (z, z/2):
            for j in (1,2,3):
                reference = phi(Decimal.from_float(argument), j)
                error = abs(Decimal.from_float(float_phi(argument,j))-reference)
                assert error <= Decimal(64)*Decimal.from_float(math.ulp(1.0))*abs(reference)
        rows.append({"z": z, "weights": observed,
                     "absolute_errors_against_75_digit_reference": errors})
    return {"status": "passed", "rows": rows,
            "scope": "Executed Python binary64 branch implementations, including half arguments for HO. Not Rust or a global floating-point proof."}


def benchmark_geometry_checks() -> dict:
    params = [("similarity-mms-v1", Decimal(1)/64, Decimal(1)/4, Decimal(3)/8),
              ("similarity-mms-v2", Decimal(1)/128, Decimal(3)/10, Decimal(21)/50)]
    rows = []
    for name, target, inner, outer in params:
        edges = []
        for radius in (inner, outer):
            x = radius**2/(2*target)
            ratio = (2*x).sqrt()*(Decimal("0.5")-x).exp()
            edges.append({"radius": str(radius), "X_at_t0_z0": str(x),
                          "uncut_swirl_amplitude_relative_to_peak": str(ratio)})
        endpoints = []
        for index in range(1,7):
            tau = target/2**index
            eta = Decimal(1)/2
            q = tau/(1-eta**2)
            # The largest spherical radius for X<=8, |eta|<=1/2 is at the edge.
            max_radius_squared = 16*q + eta**2*q**(Decimal(3)/4)
            endpoints.append({"k": index, "remaining": str(tau),
                              "full_nominal_annulus_radius_squared_max": str(max_radius_squared),
                              "nominal_annulus_inside_uncut_ball": max_radius_squared<=inner**2,
                              "equatorial_peak_radius_cells_at_N512": str(512*tau.sqrt())})
        rows.append({"case": name, "target": str(target), "edges": edges,
                     "endpoints": endpoints})
    assert all(r["nominal_annulus_inside_uncut_ball"] for r in rows[1]["endpoints"])
    ceilings = []
    for n in (128,256,512,1024):
        accepted = [k for k in range(1,32) if Decimal(n)*(Decimal(1)/128/2**k).sqrt() >= 12]
        ceilings.append({"N":n, "last_endpoint_under_radius_12_cell_screen":max(accepted,default=0)})
    assert ceilings[2]["last_endpoint_under_radius_12_cell_screen"] == 3
    return {"cases": rows, "v2_radius_screen":ceilings,
            "diagnostic_eta_limit":"1/2",
            "scope":"Analytic geometry and amplitude ratios. At t=0 the ramp makes actual velocity zero. Edge amplitude is not removed energy or an error in the localized solution. Cell counts are screening conventions, not convergence results."}


def clock_checks() -> dict:
    elapsed, remaining, dt = 1.0-2.0**-53, 2.0**-53, 2.0**-54
    e1, r1 = elapsed+dt, remaining-dt
    assert e1+r1 == 1.0
    assert Fraction.from_float(e1)+Fraction.from_float(r1) != 1
    target_ticks = 1 << 100
    elapsed_ticks, remaining_ticks, step_ticks = target_ticks-8, 8, 4
    elapsed_ticks += step_ticks
    remaining_ticks -= step_ticks
    assert elapsed_ticks+remaining_ticks == target_ticks
    assert 0 < remaining_ticks < target_ticks < 2**128
    assert step_ticks%4 == 0
    return {"binary64_dyadic_counterexample": {
                "floating_sum_equals_target":True,
                "exact_sum_error":str(Fraction.from_float(e1)+Fraction.from_float(r1)-1)},
            "u128_tick_invariant": "passed",
            "scope":"Clock arithmetic fixture; no Rust state transaction was executed."}


def symbolic_checks() -> dict:
    import sympy as s

    z = s.symbols("z", nonzero=True)
    p = [(s.exp(z)-sum(z**m/s.factorial(m) for m in range(j)))/z**j for j in (1,2,3)]
    w = [p[0]-3*p[1]+4*p[2], 2*p[1]-4*p[2], -p[1]+4*p[2]]
    closed = [(s.exp(z)*(z*z-3*z+4)-z-4)/z**3,
              2*(s.exp(z)*(z-2)+z+2)/z**3,
              (s.exp(z)*(4-z)-z*z-3*z-4)/z**3]
    assert all(s.simplify(a-b) == 0 for a,b in zip(w,closed))

    nu, lam = s.symbols("nu lam", positive=True)
    scalings = [(nu,1,nu,nu**2),
                (s.sqrt(nu),1/s.sqrt(nu),1,nu),
                (lam*s.sqrt(nu),lam/s.sqrt(nu),lam**2,lam**2*nu)]
    for alpha,beta,gamma,pressure in scalings:
        factors = [alpha*gamma,alpha**2*beta,nu*alpha*beta**2,pressure*beta]
        assert all(s.simplify(f-factors[0]) == 0 for f in factors)
    assert s.simplify(lam**2*(1-(1-lam**-2))) == 1

    X, eta, h = s.symbols("X eta h", real=True)
    a = eta+s.Rational(1,32)
    M = a*X*s.exp(-X)
    U = s.diff(M,X)
    D, L, d = s.Rational(1,2)-h, 1-2*h*eta**2, 1-eta**2
    V0 = (2*eta*X*U-2*D*eta*M-d*s.diff(M,eta))/L
    derivative_of_streamfunction = (2*D*eta*M-2*eta*X*U+d*s.diff(M,eta))/L
    assert s.simplify(V0+derivative_of_streamfunction) == 0
    assert s.limit(M,X,0) == 0 and s.limit(M,X,s.oo) == 0
    assert s.simplify(s.diff(-s.exp(-2*X)/32,X)-s.exp(-2*X)/16) == 0

    # Curl of a smooth Cartesian vector potential, with a radial swirl added.
    # Q(r^2,z,t) includes all physical cutoffs and time factors; no axis division.
    x,y,zz,t = s.symbols("x y zz t", real=True)
    R2=s.symbols("R2", real=True)
    G=s.Function("G")(R2,zz,t).subs(R2,x*x+y*y)
    B=s.Function("B")(R2,zz,t).subs(R2,x*x+y*y)
    vel=[-x*s.diff(G,zz)-y*B, -y*s.diff(G,zz)+x*B,
         2*G+x*s.diff(G,x)+y*s.diff(G,y)]
    assert s.simplify(sum(s.diff(v,c) for v,c in zip(vel,(x,y,zz)))) == 0

    # Independently check the zero-linear-operator tableau of HO(5.19).
    half=s.Rational(1,2)
    ph1,ph2,ph3=s.Integer(1),half,s.Rational(1,6)
    mat=s.zeros(5,5)
    mat[1,0]=half*ph1
    mat[2,0]=half*ph1-ph2
    mat[2,1]=ph2
    mat[3,0]=ph1-2*ph2
    mat[3,1]=mat[3,2]=ph2
    a52=half*ph2-ph3+ph2/4-ph3/2
    a54=ph2/4-a52
    mat[4,0]=half*ph1-2*a52-a54
    mat[4,1]=mat[4,2]=a52
    mat[4,3]=a54
    c=s.Matrix([0,half,half,1,half])
    b=s.Matrix([s.Rational(1,6),0,0,s.Rational(1,6),s.Rational(2,3)])
    assert mat*s.ones(5,1) == c
    conditions=[sum(b),b.dot(c),b.dot(c.applyfunc(lambda v:v**2)),b.dot(mat*c),
                b.dot(c.applyfunc(lambda v:v**3)),b.dot(s.diag(*c)*mat*c),
                b.dot(mat*c.applyfunc(lambda v:v**2)),b.dot(mat*mat*c)]
    expected=[1,half,s.Rational(1,3),s.Rational(1,6),s.Rational(1,4),
              s.Rational(1,8),s.Rational(1,12),s.Rational(1,24)]
    assert conditions == expected, (conditions,expected)

    # Nonzero-operator identities exercise every phi-dependent HO row.
    def symbolic_phi(argument, j):
        return (s.exp(argument)-sum(argument**m/s.factorial(m) for m in range(j)))/argument**j
    full=[symbolic_phi(z,j) for j in (1,2,3)]
    at_half=[symbolic_phi(z/2,j) for j in (1,2,3)]
    nonzero=s.zeros(5,5)
    nonzero[1,0]=half*at_half[0]
    nonzero[2,0]=half*at_half[0]-at_half[1]
    nonzero[2,1]=at_half[1]
    nonzero[3,0]=full[0]-2*full[1]
    nonzero[3,1]=nonzero[3,2]=full[1]
    q52=half*at_half[1]-full[2]+full[1]/4-at_half[2]/2
    q54=at_half[1]/4-q52
    nonzero[4,0]=half*at_half[0]-2*q52-q54
    nonzero[4,1]=nonzero[4,2]=q52
    nonzero[4,3]=q54
    for i in range(1,5):
        assert s.simplify(sum(nonzero[i,j] for j in range(i))-c[i]*symbolic_phi(c[i]*z,1)) == 0
    for i in (3,4):
        assert s.simplify(sum(nonzero[i,j]*c[j] for j in range(i))-c[i]**2*symbolic_phi(c[i]*z,2)) == 0
    b1=full[0]-3*full[1]+4*full[2]
    b4=-full[1]+4*full[2]
    b5=4*full[1]-8*full[2]
    assert s.simplify(b1+b4+b5-full[0]) == 0

    return {"ETD_closed_forms": "passed", "viscosity_and_periodic_scalings": "passed",
            "streamfunction_and_zero_U_moment": "passed", "radial_pressure_identity": "passed",
            "localized_Cartesian_divergence": "identically zero symbolically",
            "HO_zero_operator_order_four_conditions": "all eight conditions passed",
            "HO_nonzero_operator_rows": "four row sums, row-4/5 first moments, and output sum passed symbolically",
            "scope": "Identities for the specified manufactured profile, not Theorem 4.6 admissibility."}


def memory_checks() -> list[dict]:
    rows=[]
    for n in (128,256,512,1024):
        m=3*n//2
        h=n*n*(n//2+1)
        hp=m*m*(m//2+1)
        state=48*h
        real_vector=24*m**3
        padded_spectrum=48*hp
        # Conservative reservation: 12 retained vectors, 3 padded real vectors,
        # one padded vector spectrum, 6 retained scalar coefficient tables.
        total=12*state+3*real_vector+padded_spectrum+6*8*h
        assert state < 48*n**3
        rows.append({"N":n,"retained_vector_GiB":state/2**30,
                     "padded_real_vector_GiB":real_vector/2**30,
                     "reserved_base_GiB":total/2**30,
                     "excluded": "FFT/provider scratch, planner storage, I/O, diagnostic grids, allocator overhead"})
    return rows


def main() -> None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    with localcontext() as context:
        context.prec=75
        screen = source_screening_checks()
        result={"status":"passed","revision":"0.8","runtime_design_revision":"0.7","scales":scale_checks(),
                "coefficients":coefficient_checks(),"symbolic":symbolic_checks(),
                "source_screening":screen,
                "source_mesh_review":source_mesh_review_checks(screen),
                "floating_junctions":floating_junction_checks(),
                "benchmark_geometry":benchmark_geometry_checks(),
                "clock":clock_checks(),
                "memory":memory_checks(),
                "not_performed":["Rust compilation","PDE integration","source instance admissibility","formal proof build"]}
    text=json.dumps(result,indent=2)+"\n"
    if args.output:
        args.output.write_text(text,encoding="utf-8")
    print(text,end="")


if __name__ == "__main__":
    main()
