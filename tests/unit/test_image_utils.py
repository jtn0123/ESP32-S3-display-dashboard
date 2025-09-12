import sys
from pathlib import Path

import pytest

# Add the src directory to the Python path for imports
sys.path.append(str(Path(__file__).resolve().parents[2] / "src"))
from utils.image_utils import change_orientation


class DummyImage:
    """Simple image stub that records rotation."""

    def __init__(self):
        self.angle = None

    def rotate(self, angle):
        self.angle = angle
        return self


def test_change_orientation_known_string():
    img = DummyImage()
    change_orientation(img, "landscape")
    assert img.angle == 90


def test_change_orientation_known_int():
    img = DummyImage()
    change_orientation(img, 180)
    assert img.angle == 180


def test_change_orientation_unknown_orientation():
    img = DummyImage()
    with pytest.raises(ValueError):
        change_orientation(img, "diagonal")
