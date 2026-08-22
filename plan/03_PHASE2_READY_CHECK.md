# Phase 2 — Ready check

Objective: translate LCU ready-check events into conditional F1/F2 commands. Entry: authenticated transport evidence. LRH-003 is complete: the release watcher received a live event and printed `{"active":true,"response":"None"}`. LRH-004 is complete: the release diagnostic received F1/F2 globally and Zed's normal F1 behavior was suppressed during the 30-second capture window. Exit: endpoint routing, duplicate suppression, and ready-state hotkey cleanup pass tests. LRH-005 remains dependency-locked. Exclusion: startup registration.
