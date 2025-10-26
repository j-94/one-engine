"""Persistent storage for Graphlogue deliverables.

This module provides :class:`GraphlogueStore`, a lightweight persistence
layer that stores deliverables on disk.  Deliverables are keyed by a string
identifier and map to arbitrary string payloads.  The data is persisted in a
TSV (tab-separated values) file so it can be inspected or edited manually
when required.

Historically the persistence logic used manual string joins to serialise
rows.  That approach was brittle because any embedded tab or newline in the
payload corrupted the file format.  We now rely on :mod:`csv` with
delimiter-aware reader and writer objects, which ensures round-trips for
arbitrary string values.
"""

from __future__ import annotations

import csv
from collections import OrderedDict
from pathlib import Path
from typing import Iterable, Iterator, MutableMapping, Tuple


DeliverableItems = Iterable[Tuple[str, str]]


class GraphlogueStore:
    """Persist deliverables to disk.

    Parameters
    ----------
    path:
        Directory where the deliverables file will be stored.  The directory
        is created automatically if it does not yet exist.
    filename:
        Optional custom filename for the deliverables TSV.  Defaults to
        ``"deliverables.tsv"``.
    """

    def __init__(self, path: str | Path, filename: str = "deliverables.tsv") -> None:
        self._root = Path(path)
        self._root.mkdir(parents=True, exist_ok=True)
        self._deliverables_path = self._root / filename
        self._deliverables: "OrderedDict[str, str]" = OrderedDict(
            self._read_deliverables()
        )

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------
    @property
    def deliverables_path(self) -> Path:
        """Return the filesystem path for the persisted deliverables."""

        return self._deliverables_path

    def register_deliverable(self, name: str, value: str) -> None:
        """Persist or update a deliverable.

        Parameters
        ----------
        name:
            Identifier for the deliverable.
        value:
            Arbitrary string payload.
        """

        self._deliverables[name] = value
        self._write_deliverables()

    def get_deliverable(self, name: str) -> str | None:
        """Retrieve a deliverable value if it exists."""

        return self._deliverables.get(name)

    def iter_deliverables(self) -> Iterator[Tuple[str, str]]:
        """Iterate over deliverables in insertion order."""

        return iter(self._deliverables.items())

    def all_deliverables(self) -> MutableMapping[str, str]:
        """Return a shallow copy of the deliverables mapping."""

        return OrderedDict(self._deliverables)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------
    def _write_deliverables(self) -> None:
        """Serialise deliverables to disk using a delimiter-aware writer."""

        if not self._deliverables:
            # Remove the file if there are no deliverables stored.
            if self._deliverables_path.exists():
                self._deliverables_path.unlink()
            return

        with self._deliverables_path.open("w", encoding="utf-8", newline="") as fh:
            writer = csv.writer(
                fh,
                delimiter="\t",
                quoting=csv.QUOTE_MINIMAL,
                lineterminator="\n",
                escapechar="\\",
            )
            writer.writerows(self._deliverables.items())

    def _read_deliverables(self) -> DeliverableItems:
        """Deserialise deliverables from disk using :mod:`csv`."""

        if not self._deliverables_path.exists():
            return []

        with self._deliverables_path.open("r", encoding="utf-8", newline="") as fh:
            reader = csv.reader(
                fh,
                delimiter="\t",
                quoting=csv.QUOTE_MINIMAL,
                escapechar="\\",
            )
            rows: list[Tuple[str, str]] = []
            for row in reader:
                if not row:
                    continue
                if len(row) != 2:
                    raise ValueError(
                        "Malformed deliverable row encountered: expected two columns"
                    )
                rows.append((row[0], row[1]))
            return rows


__all__ = ["GraphlogueStore"]
