---
name: avoid-change-detector-tests
description: >-
  Use this skill when reviewing, writing, or refactoring unit tests. It helps identify and avoid "change-detector" or "mechanical" tests that merely repeat the implementation without verifying behavior.
---

# Avoid Change-Detector Tests

When writing, reviewing, or refactoring tests, ensure they verify correct behavior rather than mechanically checking the implementation structure.

## What is a Change-Detector Test?

A change-detector (or mechanical) test is a test that contains an exact copy or a direct transformation of the code under test. It acts like a checksum: it breaks in response to *any* change in the production code, without verifying the correct behavior of either the original or the modified production code.

Change detectors provide negative value:
1. They do not catch defects.
2. The added maintenance cost slows down development (e.g., having to update many mock verifications when an implementation detail changes).

## Examples of Mechanical Testing

### 1. The Absurd Line-by-Line Check
```
// Production code:
def abs(i: Int)
  return (i < 0) ? i * -1 : i

// Test code:
for (line: String in File(prod_source).read_lines())
  switch (line.number)
    1: assert line.content equals "def abs(i: Int)"
    2: assert line.content equals "  return (i < 0) ? i * -1 : i"
```
*Why it's bad:* A correct or incorrect program is equally likely to pass a test that is a derivative of the code under test.

### 2. The Heavy Mock Verification
```
// Production code:
def process(w: Work)
  firstPart.process(w)
  secondPart.process(w)

// Test code:
part1 = mock(FirstPart)
part2 = mock(SecondPart)
w = Work()
Processor(part1, part2).process(w)
verify_in_order
  was_called part1.process(w)
  was_called part2.process(w)
```
*Why it's bad:* It's tempting because it requires little thought and runs quickly, but it breaks if the implementation details (like the order or the delegation) change, even if the overall behavior remains correct.

## How to Address Them

If you find change-detector tests:
- **Rewrite them** to focus on the inputs and expected outputs (state or behavior), not the specific implementation steps.
- **Delete them** if they add no value beyond repeating the implementation.
