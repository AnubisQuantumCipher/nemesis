(module
  ;; NEMESIS built-in plugin `rail-attest` v1.
  ;;
  ;; Attests the product surface: returns the number of cockpit rails (14).
  ;; Deliberately trivial compute — the load-bearing property is the path it
  ;; travels: registered through a one-shot-reviewed governed mutation,
  ;; content-addressed by SHA-256, enabled through a second reviewed
  ;; mutation, and executed inside the bounded WASI host with an empty
  ;; capability set, metered fuel, and bounded memory.
  (func (export "attest") (result i32)
    i32.const 14))
