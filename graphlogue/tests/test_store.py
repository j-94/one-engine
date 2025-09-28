from pathlib import Path

from graphlogue.store import GraphlogueStore


def test_deliverable_round_trip_with_newlines(tmp_path: Path) -> None:
    store = GraphlogueStore(tmp_path)
    key = "report"
    value = "Line one\nLine two\nLine three"

    store.register_deliverable(key, value)

    # Recreate the store to force a read from disk and ensure the newline
    # content survives the round-trip.
    reloaded = GraphlogueStore(tmp_path)

    assert reloaded.get_deliverable(key) == value
