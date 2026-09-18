"""Centralized matrix linear algebra and target penalty utilities for UALBF verification."""

from fractions import Fraction


def exact_det(matrix):
    """Computes the exact determinant of a square matrix with integer/fraction entries

    using Gaussian elimination over fractions.Fraction.
    """
    n = len(matrix)
    if n == 0:
        return Fraction(1)
    A = [[Fraction(x) for x in row] for row in matrix]
    det = Fraction(1)
    for i in range(n):
        pivot_row = i
        while pivot_row < n and A[pivot_row][i] == 0:
            pivot_row += 1
        if pivot_row == n:
            return Fraction(0)
        if pivot_row != i:
            A[i], A[pivot_row] = A[pivot_row], A[i]
            det *= -1

        pivot = A[i][i]
        det *= pivot

        for r in range(i + 1, n):
            factor = A[r][i] / pivot
            for c in range(i, n):
                A[r][c] -= factor * A[i][c]

    return det


def mat_mul(U, B_init):
    """Multiplies two square matrices U and B_init of size n x n."""
    n = len(U)
    res = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            val = 0
            for k in range(n):
                val += U[i][k] * B_init[k][j]
            res[i][j] = val
    return res


def gram_schmidt_ortho(b):
    """Performs Gram-Schmidt Orthogonalization (GSO) over Fraction entries.

    Returns (b_star, mu) where:
      b_star: list of orthogonalized vectors (Fraction lists)
      mu: lower-triangular matrix of Fraction coefficients
    Raises ValueError if a zero-norm orthogonal vector is encountered.
    """
    n = len(b)
    if n == 0:
        return [], []
    dim = len(b[0])
    b_frac = [[Fraction(x) for x in row] for row in b]
    b_star = []
    mu = [[Fraction(0)] * n for _ in range(n)]

    for i in range(n):
        b_i_star = list(b_frac[i])
        for j in range(i):
            num = sum(b_frac[i][k] * b_star[j][k] for k in range(dim))
            den = sum(b_star[j][k] * b_star[j][k] for k in range(dim))
            if den == 0:
                raise ValueError(
                    f"Gram-Schmidt orthogonalization encountered a zero-norm vector at index {j}."
                )
            mu[i][j] = Fraction(num, den)
            for k in range(dim):
                b_i_star[k] -= mu[i][j] * b_star[j][k]
        if sum(x * x for x in b_i_star) == 0:
            raise ValueError(
                f"Gram-Schmidt orthogonalization encountered a zero-norm vector at index {i}."
            )
        b_star.append(b_i_star)

    return b_star, mu


def verify_lll_conditions(b_reduced, dim, delta=Fraction(3, 4)):
    """Verifies that the basis b_reduced is LLL-reduced according to:

    1. Size-reduction condition: |mu_{i,j}| <= 1/2 for all j < i.
    2. Lovász condition: delta * ||b_{i-1}^*||^2 <= ||b_i^*||^2 + mu_{i,i-1}^2 * ||b_{i-1}^*||^2.

    Returns (b_star, mu) on success, or raises ValueError with details on failure.
    """
    b_star, mu = gram_schmidt_ortho(b_reduced)
    n = len(b_reduced)

    # 1. Size reduction check
    for i in range(n):
        for j in range(i):
            if abs(mu[i][j]) > Fraction(1, 2):
                raise ValueError(
                    f"Basis is not LLL size-reduced: mu[{i}][{j}] = {mu[i][j]} (absolute value exceeds 1/2)."
                )

    # 2. Lovász condition check
    for i in range(1, n):
        s_prev = sum(b_star[i - 1][k] * b_star[i - 1][k] for k in range(dim))
        s_curr = sum(b_star[i][k] * b_star[i][k] for k in range(dim))
        mu_val = mu[i][i - 1]
        lhs = delta * s_prev
        rhs = s_curr + (mu_val * mu_val) * s_prev
        if lhs > rhs:
            raise ValueError(
                f"Basis violates Lovasz condition at index {i}: "
                f"delta * ||b_{i-1}^*||^2 = {lhs} > ||b_{i}^*||^2 + mu_{{{i}, {i-1}}}^2 * ||b_{i-1}^*||^2 = {rhs}."
            )

    return b_star, mu


def compute_target_penalty(m, base=1000000000.0):
    """Computes dynamic target penalty scaling factor for dimension m."""
    if m < 2 or m > 16:
        return base
    return base * (1 << (m - 2))
