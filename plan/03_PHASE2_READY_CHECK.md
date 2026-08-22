# Phase 2 — Ready check

Objective: translate LCU ready-check events into conditional F1/F2 commands. Entry: authenticated transport evidence. LRH-003 is complete: the release watcher received a live event and printed `{"active":true,"response":"None"}`. Exit: active/inactive transitions, endpoint routing, duplicate suppression, and hotkey cleanup pass tests. Tasks LRH-004–005 remain dependency-locked. Exclusion: startup registration.
