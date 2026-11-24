# pico-js: where it wins

This doc maps the workloads in which pico-js (PR #1027, engine commit `686cb9419`) is faster or slower than alien-signals 3.2.1 and `@preact/signals-core` 1.14.5. Each axis is varied while the others stay at small defaults. Every table time is the fastest of 3 or 4 processes, and `Setup` at the end states the method, statistic, and count definitions.

## 1. Creation

N computeds over one source, each read once, with no writes. Times are per computed.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| n=1 | 20 ns | 28 ns | 35 ns | pico | 0.70 |
| n=10 | 11 ns | 21 ns | 26 ns | pico | 0.54 |
| n=100 | 11 ns | 19 ns | 25 ns | pico | 0.55 |
| n=1000 | 12 ns | 21 ns | 27 ns | pico | 0.58 |
| n=10000 | 12 ns | 21 ns | 25 ns | pico | 0.59 |

- Crossover: there is none. pico-js wins at every N.
- Mechanism: a pico-js derived node is one object literal with 11 fields. alien creates a `bind` of `computedOper` over its node object, and Preact constructs a `Computed` that runs the `Signal` constructor. Retained heap is 280 B per computed in pico-js and 312 B in alien and Preact, closure and edge included.

## 2. Fan-out

One source with K reader computeds. Effects observe the first round(o·K) readers. A fraction ch of readers return a value that changes, and the rest return 0 (cutoff). Readers no effect observes are read once at setup. Each iteration is one unbatched write.

| params | pico | pico eager | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|---|
| k=1,o=0,ch=1 | 1.0 ns |  | 4.4 ns | 3.5 ns | pico | 0.30 |
| k=10,o=0,ch=1 | 1.1 ns |  | 9.1 ns | 3.5 ns | pico | 0.32 |
| k=100,o=0,ch=1 | 1.1 ns |  | 84 ns | 3.5 ns | pico | 0.32 |
| k=1000,o=0,ch=1 | 1.3 ns |  | 1.89 µs | 3.4 ns | pico | 0.37 |
| k=10000,o=0,ch=1 | 1.2 ns |  | 16.3 µs | 3.5 ns | pico | 0.35 |
| k=1,o=0.1,ch=1 | 1.0 ns | 1.2 ns | 4.5 ns | 3.4 ns | pico | 0.30 |
| k=10,o=0.1,ch=1 | 43 ns | 45 ns | 45 ns | 31 ns | Preact | 1.39 |
| k=100,o=0.1,ch=1 | 397 ns | 361 ns | 447 ns | 286 ns | Preact | 1.26 |
| k=1000,o=0.1,ch=1 | 5.10 µs | 5.31 µs | 7.61 µs | 2.77 µs | Preact | 1.84 |
| k=10000,o=0.1,ch=1 | 53.3 µs | 52.1 µs | 59.1 µs | 33.8 µs | Preact | 1.54 |
| k=1,o=1,ch=1 | 36 ns | 42 ns | 25 ns | 31 ns | alien | 1.42 |
| k=10,o=1,ch=1 | 312 ns | 262 ns | 257 ns | 278 ns | tie alien, pico eager | 1.02 |
| k=100,o=1,ch=1 | 3.08 µs | 2.75 µs | 2.45 µs | 2.97 µs | alien | 1.12 |
| k=1000,o=1,ch=1 | 37.6 µs | 36.4 µs | 30.9 µs | 37.1 µs | alien | 1.18 |
| k=10000,o=1,ch=1 | 377 µs | 372 µs | 304 µs | 380 µs | alien | 1.22 |

| params | pico | pico eager | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|---|
| k=100,o=1,ch=0 | 1.72 µs | 683 ns | 1.44 µs | 1.42 µs | pico eager | 0.48 |
| k=100,o=1,ch=0.1 | 2.05 µs | 1.19 µs | 1.81 µs | 1.53 µs | pico eager | 0.78 |
| k=100,o=1,ch=0.5 | 2.70 µs | 2.08 µs | 2.32 µs | 2.46 µs | pico eager | 0.90 |
| k=10000,o=1,ch=0 | 232 µs | 108 µs | 201 µs | 217 µs | pico eager | 0.54 |
| k=10000,o=1,ch=0.1 | 282 µs | 193 µs | 231 µs | 227 µs | pico eager | 0.85 |
| k=10000,o=1,ch=0.5 | 340 µs | 286 µs | 309 µs | 325 µs | pico eager | 0.93 |

| cell | engine | mark | verify | tmp |
|---|---|---|---|---|
| k=10000,o=0 | pico | 0 | 0 | 0 |
| k=10000,o=0 | alien | 10000 | 0 | 0 |
| k=10000,o=0 | Preact | 0 | 0 | 0 |
| k=10000,o=0.1 | pico | 12000 | 1000 | 4 |
| k=10000,o=0.1 | alien | 21000 | 1001 | 1 |
| k=10000,o=0.1 | Preact | 2000 | 2000 | 0 |
| k=10000,o=1 | pico | 30000 | 10000 | 4 |
| k=10000,o=1 | pico eager | 20000 | 0 | 3 |
| k=10000,o=1 | alien | 30000 | 10001 | 1 |
| k=10000,o=1 | Preact | 20000 | 20000 | 0 |

- Crossover in o: pico-js wins at o=0 at every K. Preact wins from o=0.1 at K of 10 or more. alien wins at o=1.
- Crossover in ch, with o=1: pico eager wins for ch of 0.5 or less, and alien wins at ch=1. The crossover lies between ch=0.5 and ch=1 at both K=100 and K=10000.
- Mechanism at o=0: the first write leaves every reader Dirty and sets `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`, so each later write returns after one flag check. Preact does the same work with a version increment. alien's `propagate` visits all K subscribers on every write.
- Mechanism at o=0.1: Preact never subscribes an unobserved computed to the source, so a write visits only the observed readers. pico-js and alien keep every reader in the source's dependents list, so a write visits all K. The readers that are observed re-read the source, which resets pico-js's all-Dirty flag. pico-js and alien also retain unobserved readers that the program dropped (2.7 MB for K=10000), while Preact lets them be collected (53 KB).
- Mechanism at o=1, ch=1: the operation counts are equal (30000 marks against alien's 30000). alien is 1.2 times faster at K=10000 on per-node constants: a pico-js effect runs as an `onDirty` closure that calls `pull`, which then runs `verify` on one edge. Each flushing pico-js write also allocates a queue of 3 objects plus a marking stack (tmp=4), where alien reuses one global queue (tmp=1).
- Mechanism with cutoff, eager: an eager reader recomputes in the flush and marks its dependents only if its value changed, so the effects of unchanged readers are never queued. Lazy pico-js and both competitors notify every effect and verify it back to Clean.

## 3. Fan-in

One computed reads F sources, and one source is written. "Observed" means an effect reads the computed. Otherwise the iteration pulls the computed after the write.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| f=1,observed=true | 39 ns | 27 ns | 31 ns | alien | 1.48 |
| f=10,observed=true | 57 ns | 82 ns | 74 ns | pico | 0.77 |
| f=100,observed=true | 241 ns | 608 ns | 515 ns | pico | 0.47 |
| f=1000,observed=true | 2.16 µs | 6.12 µs | 6.39 µs | pico | 0.35 |
| f=1,observed=false | 9.6 ns | 15 ns | 16 ns | pico | 0.63 |
| f=10,observed=false | 27 ns | 64 ns | 78 ns | pico | 0.43 |
| f=100,observed=false | 177 ns | 515 ns | 487 ns | pico | 0.36 |
| f=1000,observed=false | 1.86 µs | 5.37 µs | 5.90 µs | pico | 0.35 |

- Crossover: alien wins at F=1 when observed, because of pico-js's fixed cost per flushing write. pico-js wins from F=10 on, and it wins at every F when unobserved.
- Mechanism: a recompute re-reads all F sources in every engine, and none of them allocates edges. pico-js `readSource` reuses the edge at the read position after one comparison of `next.dependency`. alien `link` compares `depsTail`, `nextDep`, and the version on each read. Preact walks the source list twice more per recompute (`prepareSources` and `cleanupSources`) and runs `addDependency` with its rollback node on each read.

## 4. Depth

A chain of D computeds from one source, with an effect at the end. The cut node c returns 0: c is none, the first node (next to the source), the middle node, or the last node (next to the effect).

| params | pico | pico eager | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|---|
| d=1,c=none | 37 ns | 40 ns | 26 ns | 30 ns | alien | 1.45 |
| d=1,c=first | 23 ns | 18 ns | 22 ns | 17 ns | Preact | 1.07 |
| d=10,c=none | 134 ns | 130 ns | 164 ns | 154 ns | pico eager | 0.85 |
| d=10,c=first | 66 ns | 18 ns | 67 ns | 66 ns | pico eager | 0.27 |
| d=10,c=mid | 91 ns | 54 ns | 109 ns | 96 ns | pico eager | 0.56 |
| d=10,c=last | 122 ns | 106 ns | 166 ns | 154 ns | pico eager | 0.69 |
| d=100,c=none | 1.15 µs | 1.07 µs | 1.49 µs | 1.72 µs | pico eager | 0.72 |
| d=100,c=first | 574 ns | 18 ns | 534 ns | 788 ns | pico eager | 0.03 |
| d=100,c=mid | 881 ns | 527 ns | 985 ns | 1.17 µs | pico eager | 0.53 |
| d=100,c=last | 1.15 µs | 990 ns | 1.43 µs | 1.71 µs | pico eager | 0.69 |
| d=1000,c=none | 13.7 µs | 12.2 µs | 21.6 µs | 30.6 µs | pico eager | 0.56 |
| d=1000,c=first | 12.0 µs | 20 ns | 10.6 µs | 18.8 µs | pico eager | 0.00 |
| d=1000,c=mid | 14.5 µs | 5.22 µs | 14.3 µs | 19.5 µs | pico eager | 0.37 |
| d=1000,c=last | 16.8 µs | 10.6 µs | 21.5 µs | 25.4 µs | pico eager | 0.49 |

| cell | engine | mark | verify | recomputes | tmp |
|---|---|---|---|---|---|
| d=1000,c=none | pico | 2001 | 1000 | 1000 | 3 |
| d=1000,c=none | pico eager | 1001 | 0 | 1000 | 3 |
| d=1000,c=none | alien | 1001 | 1001 | 1000 | 1000 |
| d=1000,c=none | Preact | 1001 | 1001 | 1000 | 0 |
| d=1000,c=first | pico | 1001 | 1000 | 1 | 3 |
| d=1000,c=first | pico eager | 1 | 0 | 1 | 3 |
| d=1000,c=first | alien | 1001 | 1001 | 1 | 1000 |
| d=1000,c=first | Preact | 1001 | 1001 | 1 | 0 |

- Crossover: alien wins at D=1. Pico eager wins at D of 10 or more for every cut. Lazy pico-js beats both competitors at D of 10 or more with c=none or c=last, ties or beats them with c=mid, and loses to alien by 7 to 13% with c=first at D of 100 or more.
- Mechanism, eager: marking stops at an eager node. The flush recomputes the chain one node at a time, and a cutoff ends the walk, so c=first costs one recompute and one mark (20 ns at D=1000).
- Mechanism, lazy: a write marks the whole chain (D marks for Check), and `verify` walks it without recursion. alien allocates one `checkDirty` stack frame per level (tmp=1000). Preact recurses through `_refresh` and `needsToRecompute`, which is why it falls behind as D grows (30.6 µs against 13.7 µs at D=1000).

## 5. Repeated writes

M writes to one source, which one computed reads. "Observed" means an effect reads the computed. Otherwise the iteration pulls the computed once after the M writes.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| m=1,batched=false,observed=true | 36 ns | 25 ns | 29 ns | alien | 1.42 |
| m=10,batched=false,observed=true | 360 ns | 254 ns | 319 ns | alien | 1.42 |
| m=100,batched=false,observed=true | 3.51 µs | 2.44 µs | 3.01 µs | alien | 1.44 |
| m=1,batched=true,observed=true | 37 ns | 25 ns | 39 ns | alien | 1.52 |
| m=10,batched=true,observed=true | 45 ns | 52 ns | 64 ns | pico | 0.87 |
| m=100,batched=true,observed=true | 134 ns | 282 ns | 302 ns | pico | 0.48 |
| m=1,batched=false,observed=false | 9.5 ns | 15 ns | 15 ns | pico | 0.64 |
| m=10,batched=false,observed=false | 19 ns | 38 ns | 43 ns | pico | 0.49 |
| m=100,batched=false,observed=false | 103 ns | 271 ns | 348 ns | pico | 0.38 |
| m=1,batched=true,observed=false | 15 ns | 15 ns | 25 ns | tie pico, alien | 0.96 |
| m=10,batched=true,observed=false | 22 ns | 36 ns | 44 ns | pico | 0.60 |
| m=100,batched=true,observed=false | 106 ns | 232 ns | 196 ns | pico | 0.54 |

| cell | engine | mark | tmp |
|---|---|---|---|
| m=100,batched,observed | pico | 3 | 3 |
| m=100,batched,observed | alien | 101 | 1 |
| m=100,batched,observed | Preact | 101 | 0 |
| m=100,unbatched,observed | pico | 300 | 300 |
| m=100,unbatched,observed | alien | 200 | 100 |
| m=100,unbatched,unobserved | pico | 1 | 0 |
| m=100,unbatched,unobserved | alien | 100 | 0 |
| m=100,unbatched,unobserved | Preact | 0 | 0 |

- Crossover, batched and observed: alien wins at M=1 and pico-js wins from M=10 on.
- Crossover, unbatched and observed: there is none. alien wins at every M, by 1.4 times.
- Unobserved: pico-js wins at every M, batched or not, and ties alien only at M=1 batched.
- Mechanism: after the first write in a batch, the source's dependents are known to be all Dirty, so every later write is one store and one flag check (3 marks total at M=100). alien and Preact walk the subscribers on every write. Unbatched writes with an effect flush on every write in all engines, and each flushing pico-js write allocates a 3-object queue (tmp=300 at M=100) and makes 3 marking visits where alien makes 2. A `startBatch` of one write allocates `Mutators` plus 2 arrays, which is the 37 ns against 25 ns at M=1.

## 6. Dynamic dependencies

N computeds each read a shared `tick` source, then 4 sources of branch a or branch b. A fraction p of the computeds switch branches on every write. Each computed has an effect, and each iteration writes `tick`.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| n=100,p=0 | 4.06 µs | 4.19 µs | 6.01 µs | tie pico, alien | 0.97 |
| n=100,p=0.1 | 4.18 µs | 4.79 µs | 6.93 µs | pico | 0.87 |
| n=100,p=0.5 | 5.16 µs | 6.41 µs | 9.87 µs | pico | 0.80 |
| n=100,p=1 | 5.08 µs | 8.57 µs | 12.6 µs | pico | 0.59 |
| n=1000,p=0 | 54.0 µs | 52.0 µs | 70.3 µs | tie alien, pico | 1.04 |
| n=1000,p=0.1 | 57.9 µs | 63.3 µs | 80.2 µs | pico | 0.91 |
| n=1000,p=0.5 | 64.3 µs | 88.6 µs | 113 µs | pico | 0.73 |
| n=1000,p=1 | 69.2 µs | 113 µs | 135 µs | pico | 0.61 |

- Crossover: alien and pico-js tie at p=0, and pico-js wins from p=0.1 on. The margin grows with p (0.59 to 0.61 times the competitor's time at p=1).
- Mechanism: pico-js `readSource` moves the edge at a read position to the new source in place (unlink, then relink), so a switch allocates nothing (edges=0). alien `link` allocates a new link and `purgeDeps` unlinks the old one (4000 edges per iteration at N=1000, p=1). Preact allocates a new `Node` per changed dependency and unsubscribes the old one in `cleanupSources` (4000 as well).

## 7. Churn

Each iteration creates N computeds over one long-lived source, attaches an effect to each, then disposes of them. In dispose mode, pico-js calls `disposeDerivedNode` on each effect node and each computed, while alien and Preact call the effect's disposer. In drop mode, pico-js drops the nodes without disposing them. alien and Preact have no drop variant, because their dispose mode already only unsubscribes the effects.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| n=10,mode=dispose | 788 ns | 511 ns | 786 ns | alien | 1.54 |
| n=10,mode=drop | 762 ns |  |  | tie pico |  |
| n=100,mode=dispose | 8.11 µs | 4.89 µs | 7.06 µs | alien | 1.66 |
| n=100,mode=drop | 7.42 µs |  |  | tie pico |  |
| n=1000,mode=dispose | 83.5 µs | 50.1 µs | 72.8 µs | alien | 1.67 |
| n=1000,mode=drop | 74.1 µs |  |  | tie pico |  |

| cell | engine | retained B/iteration | one write after 73 iterations |
|---|---|---|---|
| n=10,drop | pico | 7394 | 730 effects run, 0.50 ms |
| n=100,drop | pico | 70412 | 7300 effects run, 1.66 ms |
| n=1000,drop | pico | 704497 | 73000 effects run, 9.60 ms |
| n=10 to 1000,dispose | each engine | no growth (noise within 12 KB) | 0 effects run |

- Crossover: there is none. alien wins at every N, by 1.5 to 1.7 times. pico-js and Preact are within 15% of each other.
- Mechanism: the counts are equal across engines (2 edges and 2 computations per pair). The difference is creation and teardown constant: a pico-js effect is a derived node, an `onDirty` closure, and a `pull`, and disposal is two calls that each unlink a list. An alien effect is one object, and its disposer unlinks the effect, and `unwatched` then unlinks the computed.
- Drop mode: a dropped pico-js computed stays in the source's dependents list, and its effect node stays in the computed's list. They are never collected, every later write marks them, and their `onDirty` callbacks run.

## 8. Nodes nobody observes

The graph has 100 sources, 1000 first-level computeds that each read 3 sources, 100 second-level computeds that each read 10 first-level ones, and a root that reads the 100 second-level computeds. Setup reads the root once, with no effects. Each iteration does W unbatched writes to distinct sources, then reads the root.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| w=1 | 2.74 µs | 4.19 µs | 12.8 µs | pico | 0.65 |
| w=10 | 11.7 µs | 18.3 µs | 24.9 µs | pico | 0.64 |
| w=100 | 28.9 µs | 51.6 µs | 30.1 µs | tie pico, Preact | 0.96 |
| w=1000 | 30.6 µs | 126 µs | 33.6 µs | pico | 0.91 |

| cell | engine | mark | verify | recomputes |
|---|---|---|---|---|
| w=1 | pico | 145 | 151 | 58.5 |
| w=1 | alien | 147 | 153 | 58.5 |
| w=1 | Preact | 0 | 3846 | 58.5 |
| w=1000 | pico | 5200 | 101 | 1101 |
| w=1000 | alien | 35099 | 115 | 1101 |
| w=1000 | Preact | 0 | 1101 | 1101 |

- Crossover: pico-js wins or ties at every W. Preact closes to within 4 to 10% at W of 100 or more, where every node recomputes anyway.
- Mechanism, small W: Preact keeps no dependents for unobserved computeds, so a write costs nothing, but a read checks the versions of the whole graph (3846 edges for one write). pico-js and alien mark only the written source's downstream nodes and verify only those.
- Mechanism, large W: a pico-js source whose dependents were all left Dirty skips marking, so marking stays at 5200 visits however large W gets. alien's `propagate` revisits the subscribers on each write (35099 visits at W=1000).

## 9. Effects

The graph has 10 sources and 100 computeds that each read 2 sources. E effects each read 3 computeds. Each iteration writes 5 sources, either in one batch or as 5 unbatched writes.

| params | pico | alien | Preact | winner | pico ÷ best of alien, Preact |
|---|---|---|---|---|---|
| e=1,batched=false | 205 ns | 213 ns | 280 ns | tie pico, alien | 0.96 |
| e=10,batched=false | 1.33 µs | 1.42 µs | 1.74 µs | pico | 0.94 |
| e=100,batched=false | 8.34 µs | 9.42 µs | 12.7 µs | pico | 0.88 |
| e=1000,batched=false | 108 µs | 90.5 µs | 113 µs | alien | 1.20 |
| e=1,batched=true | 73 ns | 100 ns | 131 ns | pico | 0.73 |
| e=10,batched=true | 566 ns | 712 ns | 806 ns | pico | 0.79 |
| e=100,batched=true | 3.30 µs | 4.63 µs | 5.39 µs | pico | 0.71 |
| e=1000,batched=true | 47.9 µs | 37.2 µs | 47.2 µs | alien | 1.29 |

| cell | engine | mark | verify | tmp |
|---|---|---|---|---|
| e=1000,batched | pico | 3700 | 33 | 7 |
| e=1000,batched | alien | 3800 | 34 | 61 |
| e=1000,batched | Preact | 1900 | 1670 | 0 |

- Crossover, batched: pico-js wins at E from 1 to 100, and alien wins at E=1000. The crossover lies between E=100 and E=1000.
- Crossover, unbatched: pico-js wins or ties at E up to 100, and alien wins at E=1000.
- Mechanism: at small E, the fixed batch cost and the marking of the 100 computeds dominate, and pico-js's dependents-known-Dirty skip removes repeated marking within a batch. At E=1000, the cost per effect run dominates with near-equal counts, and the per-node constant described under fan-out favors alien.

## Crossovers

- Fan-out, observed fraction o: pico-js wins at o=0 at every K. Preact wins from o=0.1 once K is 10 or more. alien wins at o=1 at every K except K=10, where it ties pico eager.
- Fan-out, changed fraction ch, with o=1: pico eager wins for ch of 0.5 or less, and alien wins at ch=1, at both K=100 and K=10000.
- Fan-in: alien wins at F=1 when observed, and pico-js wins from F=10 on.
- Depth: alien wins at D=1, and pico eager wins from D=10 on for every cut.
- Repeated writes with an effect, batched: alien wins at M=1, and pico-js wins from M=10 on. Unbatched: alien wins at every M.
- Dynamic dependencies: pico-js and alien tie at p=0, and pico-js wins from p=0.1 on.
- Effects, batched: pico-js wins up to E=100, and alien wins at E=1000.
- Churn, nodes nobody observes, and creation have no crossover in the measured range. alien wins churn, and pico-js wins the other two.

## Summary

pico-js is faster when:

- the program creates nodes: a computed plus its first read costs 11 to 13 ns, against 19 to 21 ns (alien) and 25 to 27 ns (Preact), at every N from 10 to 10000;
- a node reads many sources: at F of 10 or more, pico-js runs 1.3 to 2.9 times faster than either competitor (0.35 times their time at F=1000), observed or not;
- a source is written repeatedly before anything reads it: M of 10 or more unobserved writes run 1.7 to 2.6 times faster, and M of 10 or more writes inside one batch with an effect run 1.15 to 2.1 times faster;
- a graph is read without effects and then written: at W from 1 to 1000 writes, pico-js runs 1.5 to 4.1 times faster than alien and ties or beats Preact (4.7 times faster at W=1, within 4 to 10% at W of 100 or more);
- nodes on the path to an effect are eager and a value stops changing (cutoff): an eager pico-js chain of D of 10 or more beats both competitors by 1.2 to 530 times, and fan-out with 50% or fewer changing readers is 1.07 to 2.1 times faster;
- dependencies switch: with p of 10% or more of 100 to 1000 nodes switching branches, pico-js runs 1.1 to 1.7 times faster;
- effects share a graph and the write is batched, for E up to 100 effects (1.3 to 1.4 times faster);
- readers exist that no effect observes and nothing reads again (o=0): a write costs 1.0 to 1.3 ns at any K, against 3.5 ns (Preact) and up to 16 µs (alien at K=10000).

pico-js is slower when:

- most readers of a source are observed and every value changes: alien is 1.1 to 1.4 times faster at K=1 and at K from 100 to 10000, and ties at K=10, because the per-node constant is higher in pico-js at equal operation counts;
- 10% of a fan-out of K of 10 or more is observed and the unobserved readers have been read once: Preact is 1.3 to 1.8 times faster, because Preact does not link a computed to its sources until something subscribes to it, while pico-js walks every reader on every write;
- each write flushes on its own with an effect attached: unbatched writes (single-call `writeSource`) are 1.4 times slower than alien at every M, and a batch of a single write is 1.5 times slower;
- more than about 100 effects run per flush: at E=1000, alien is 1.2 to 1.3 times faster;
- nodes are created, observed, and disposed (churn): alien is 1.5 to 1.7 times faster at N from 10 to 1000, and Preact is 1.0 to 1.15 times faster;
- the graph is tiny and an effect is attached (D=1, F=1, K=1): alien wins by 1.4 to 1.5 times, because pico-js pays a fixed cost per flushing write;
- a chain cuts off next to its source and pico-js keeps the chain lazy: alien beats lazy pico-js by 7 to 13% at D of 100 or more (pico eager wins that case by 530 times at D=1000).

Dropping pico-js nodes without `disposeDerivedNode` saves 3 to 11% of churn time and costs 704 bytes retained per computed and effect pair. The dropped effects' `onDirty` callbacks keep running: one write after 73 churn iterations of N=1000 ran 73000 dropped effects in 9.6 ms.

## Setup

- Machine: Apple M5 Pro, 18 cores, Node v24.12.0.
- Code: the harness is at `scratchpad/where-it-wins/` (`cells.mjs` lists the matrix, `src/wl-*.ts` hold one workload file per engine written against that engine's own API, and `src/driver.ts` times them). The timed builds bundle the published code: pico-js `src` at `686cb9419`, alien `esm/index.mjs`, and Preact `dist/signals-core.mjs`. The counting builds bundle copies that `patch.mjs` instruments.
- Effects: alien and Preact use their native `effect`. A pico-js effect is a lazy derived node whose `onDirty` calls `pull` on it, so the flush of the top-level `writeSource` or of `startBatch` runs that callback. The "pico eager" column makes the derived nodes an effect reads eager (fan-out: the observed readers; depth: every chain node).
- Writes: unbatched writes are pico-js `writeSource` (a batch of one), alien `s(v)`, and Preact `s.value = v`. Batched writes are `startBatch`, alien `startBatch`/`endBatch`, and Preact `batch`.
- Timing: each process runs one cell under one engine. It warms up for 200 ms, calibrates repetitions to 40 ms rounds, and runs 12 rounds; churn uses 60 ms and 10 rounds of 10 ms. A `PerformanceObserver` records GC entries inside the rounds. Processes are interleaved in an order shuffled per rep, and each one holds the shared bench lock after the 1-minute load average drops below 4.
- Reps: 3 complete reps ran, plus 98 jobs of a fourth, so every cell has 3 or 4 processes per engine. External load held the gate closed for about 2 hours, and the run stopped there.
- Statistic: a table time is the fastest process's median round. Rounds inside a process vary by under 5%. Whole processes are bimodal, though: 10 to 20% of them ran 1.5 to 4 times slower under the background load, for every engine. With 3 processes, a median picks that slow mode when 2 of 3 hit it. The median-of-processes statistic changes the winner in 7 of 86 cells, all within 20%: fan-out K=10000 ch=0.5, repeat m=1 batched unobserved, dynamic n=1000 p=0 and p=0.5, nobody w=100, and effects e=1 and e=10 unbatched.
- Winner: an engine wins when its time is at least 5% below every other engine's. Pico eager counts as pico-js. Otherwise the cell is a tie between the engines within 5%. The last column is pico-js's best time (lazy or eager) divided by the better of alien and Preact.
- Counts, per iteration, from the counting builds: `mark` counts dependents visited while marking or propagating (pico-js `markDirty`, `markCheck`, and `markCheckDependentsDirty`; alien `propagate` and `shallowPropagate`; Preact `_notify` calls). `verify` counts dependency edges examined to decide whether to recompute (pico-js `verify`, alien `checkDirty`, Preact `needsToRecompute`). `edges` counts edge objects allocated (pico-js edges, alien links, Preact `Node`s). `tmp` counts transient allocations: traversal stacks, stack frames, and batch queues. In the counting runs, the effect sinks of all engines are equal in every cell, which checks that they do the same work.
