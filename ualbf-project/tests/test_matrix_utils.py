from fractions import Fraction
import pytest

from matrix_utils import (
    exact_det,
    mat_mul,
    gram_schmidt_ortho,
    verify_lll_conditions,
    compute_target_penalty,
)


def test_exact_det_basic():
    # Empty
    assert exact_det([]) == Fraction(1)
    # 1x1
    assert exact_det([[7]]) == Fraction(7)
    # 2x2 identity
    assert exact_det([[1, 0], [0, 1]]) == Fraction(1)
    # 2x2 unimodular det=1
    assert exact_det([[1, 2], [3, 7]]) == Fraction(1)
    # 2x2 matrix with row swap det=-1
    assert exact_det([[0, 1], [1, 0]]) == Fraction(-1)
    # 2x2 singular
    assert exact_det([[1, 2], [2, 4]]) == Fraction(0)


def test_exact_det_fractions():
    matrix = [[Fraction(1, 2), Fraction(1, 3)], [Fraction(1, 4), Fraction(1, 5)]]
    # (1/2)*(1/5) - (1/3)*(1/4) = 1/10 - 1/12 = 1/60
    assert exact_det(matrix) == Fraction(1, 60)


def test_mat_mul_basic():
    A = [[1, 2], [3, 4]]
    B = [[5, 6], [7, 8]]
    # [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19, 22], [43, 50]]
    expected = [[19, 22], [43, 50]]
    assert mat_mul(A, B) == expected


def test_gram_schmidt_ortho():
    # Standard 2D orthogonal basis
    b = [[1, 0], [0, 1]]
    b_star, mu = gram_schmidt_ortho(b)
    assert b_star == [[Fraction(1), Fraction(0)], [Fraction(0), Fraction(1)]]
    assert mu[1][0] == Fraction(0)

    # Linearly dependent vectors raise ValueError
    dep_b = [[1, 2], [2, 4]]
    with pytest.raises(ValueError, match="zero-norm vector"):
        gram_schmidt_ortho(dep_b)


def test_verify_lll_conditions():
    # Identity basis is LLL reduced
    b_id = [[1, 0], [0, 1]]
    b_star, mu = verify_lll_conditions(b_id, 2)
    assert len(b_star) == 2

    # Non-size-reduced basis (mu[1][0] = 2 > 1/2)
    non_reduced = [[1, 0], [2, 1]]
    with pytest.raises(ValueError, match="not LLL size-reduced"):
        verify_lll_conditions(non_reduced, 2)


def test_compute_target_penalty():
    base = 1e9
    assert compute_target_penalty(1, base) == base
    assert compute_target_penalty(2, base) == base
    assert compute_target_penalty(3, base) == base * 2
    assert compute_target_penalty(4, base) == base * 4
    assert compute_target_penalty(16, base) == base * 16384
    assert compute_target_penalty(17, base) == base
