"""Fast contracts for the bounded current-grid arithmetic pilot."""
import unittest
from reference.reference_arithmetic_pilot import category, centered_numerator, full_grid_counts


class ReferenceArithmeticPilot(unittest.TestCase):
    """Keep exact input classes and corrected full-grid counts stable."""

    def test_centering_and_representative_classes_are_exact(self) -> None:
        self.assertEqual(tuple(centered_numerator(value) for value in range(12)),
                         (0, 1, 2, 3, 4, 5, -6, -5, -4, -3, -2, -1))
        self.assertEqual(category((0, 0, 1), 64), "active")
        self.assertEqual(category((5, 0, 0), 64), "active")
        self.assertEqual(category((1, 1, 1), 64), "active")
        self.assertEqual(category((6, 0, 0), 64), "exterior")
        self.assertEqual(category((6, 0, 0), 0), "startup")

    def test_full_grid_union_counts_overlap_once(self) -> None:
        self.assertEqual(full_grid_counts(), {
            "rows": 5184, "exterior_spatial": 1213, "exterior_rows": 3639,
            "startup_rows": 1728, "startup_exterior_intersection": 1213,
            "shortcut_union_rows": 4154, "active_rows": 1030,
        })


if __name__ == "__main__":
    unittest.main()
