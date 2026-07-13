# Computers {#computers}

> **Test article.** This original, Wikipedia-style sample is deliberately structured to exercise a Markdown reader: headings, anchors, tables, lists, code, links and footnotes.

Computers are programmable machines that transform data according to a sequence of instructions. A modern laptop combines processing, memory, storage, input, output and networking in one compact system. For adjacent topics, see [Art](art.md) and [Italy](italy.md).

---

## Contents

1. [A short history](#history)
2. [Computer architecture](#computer-architecture)
3. [Software](#software)
4. [Kinds of computer](#kinds-of-computer)
5. [Glossary](#glossary)

## A short history {#history}

The word *computer* originally described a person who performed calculations. Mechanical calculators appeared centuries before electronic machines. In the twentieth century, relays, vacuum tubes and then transistors made automatic calculation faster and more reliable.

Important shifts included:

- **Stored programs:** instructions and data could share the same memory.
- **Integrated circuits:** many electronic components could be manufactured on a small chip.
- **Personal computing:** computers became ordinary tools for homes, schools and offices.
- **Networks:** machines could exchange information and share services across long distances.

| Era | Typical technology | Everyday effect |
| --- | --- | --- |
| Mechanical | gears, levers, punched cards | repeatable arithmetic |
| Electronic | vacuum tubes and relays | high-speed calculation |
| Semiconductor | transistors and integrated circuits | smaller, cheaper machines |
| Networked | internet protocols and cloud services | shared information and remote work |

## Computer architecture {#computer-architecture}

At a high level, a computer repeatedly fetches an instruction, interprets it, performs an operation, and records a result. This cycle is usually measured in billions of steps per second.

### Core components

| Component | Main role | Example question it answers |
| --- | --- | --- |
| Central processing unit (CPU) | executes instructions | “What calculation happens next?” |
| Random-access memory (RAM) | holds active data | “What is needed right now?” |
| Storage | retains data when powered off | “What should still exist tomorrow?” |
| Input devices | bring information in | “What did the user select?” |
| Output devices | present results | “What should the user see or hear?” |
| Network adapter | exchanges data | “Where should this message go?” |

### The instruction cycle

```text
fetch instruction → decode instruction → execute operation → store result
```

The details vary by processor design, but the broad idea is useful when reading about performance. A faster CPU is not always faster for every task: memory speed, storage latency, graphics hardware and the program itself matter too.

> A computer is best understood as a system. Its speed and usefulness depend on how the parts work together, not on one specification alone.

## Software {#software}

Software is the set of instructions that tells hardware what to do. A program may be close to the machine, such as a device driver, or close to the user, such as a web browser.

- [x] An operating system manages hardware resources.
- [x] Applications help people complete a specific task.
- [ ] A program is not automatically secure merely because it is popular.

### A tiny example

```rust
fn main() {
    let message = "Hello, computer";
    println!("{message}");
}
```

This is a complete Rust program: it stores text in a variable and writes it to standard output.

## Kinds of computer {#kinds-of-computer}

The boundaries are flexible. A phone is a computer, as are servers in data centres, embedded controllers in appliances and scientific machines used for simulations.

| Type | Often optimized for | Typical form |
| --- | --- | --- |
| Desktop | modularity and sustained performance | separate screen and case |
| Laptop | portability and battery life | folding all-in-one device |
| Server | reliability and many simultaneous users | rack or cloud instance |
| Embedded system | one narrow task | controller inside another product |
| Supercomputer | very large simulations | coordinated cluster of systems |

## Glossary {#glossary}

**Algorithm**
: A method for completing a problem or transformation.

**Bit**
: A binary digit, usually represented as `0` or `1`.

**Byte**
: A small unit of data commonly made of eight bits.[^byte]

[^byte]: The exact relationship between bits, bytes and larger storage units is a useful topic for a longer article.

---

Continue with [Art](art.md) or [Italy](italy.md). Return to the [top](#computers).
