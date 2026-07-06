# Tholos

Bonded assertion and dispute oracle for resolving real world outcomes. Resolution infra for prediction markets and anything else that needs a trustworthy yes/no.

## Status

Early design phase. No contracts shipped yet.

## Overview

Prediction markets and similar products eventually need to answer a hard question: who decides what actually happened? Existing approaches either rely on token holder votes that can be captured by large holders with a stake in the outcome, or on a centralized, regulated party acting as sole resolver.

Tholos is a bonded assertion and dispute contract: anyone can propose an outcome by posting a bond, and a challenge window gives others the chance to dispute it before it finalizes. It is designed to be standalone and composable, so any contract that needs a trustworthy resolution of a real world event can plug into it rather than building its own oracle logic.

## License

TBD
