# 10 — Ports

Additional Host or Peer **implementations** are allowed by law.  
This framework’s *reference* kernels stay Rust.

## When to add a Peer port

| Surface | Example port |
|---------|----------------|
| Browser DOM | TypeScript Peer later |
| Agent / server already in Python | Python Peer port (not Host) |
| MCU / device | C/Rust embedded Peer with tiny profile |

Each port:

1. Declares a profile  
2. Passes applicable vector families  
3. Never mints root Caps  

## Host ports

A second Host language is **out of scope** for this framework’s reference path (user choice: Rust only).  
If added later: same vectors, same Cap binds, explicit cross-Host trust policy.

## L7 callers

Any language may hold a Cap and call Host `submit` over IPC/HTTP.  
That does not make them kernels.
