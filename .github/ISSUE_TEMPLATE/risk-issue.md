---
name: Risk issue
about: Track Risk issues
title: "[Risk]"
labels: 'high-impact,medium-impact,low-impact,informational,unknown-impact,high-likelihood,medium-likelihood,low-likelihood,unknown-likelihood,unsolved,mitigated,wontfix,fixed,not-an-issue,risk'
assignees: ''

---

* **Impact:** use issue labels for impact such as `high-impact` / `medium-impact` / `low-impact` / `informational` / `unknown-impact`
* **Likelihood:** use issue labels for likelihood such as `high-likelihood` / `medium-likelihood` / `low-likelihood` / `unknown-likelihood`
* **Status:** use issue label for status such as `fixed` / `mitigated` / `unsolved` / `wontfix` / `not-an-issue`
* **Fix or mitigation updates:** Please write in the issue comments any updates.

## Short Description

..

## Detailed Description

...

## Scenario

1. ..
2. ..
3. ...

## Possible Fixes or Mitigations

1. ..
2. ..

## Nomenclature:

Each risk should have a likelihood level (use **Labels** for level choice `risk:<insert>-impact` and `risk:<insert>-likehood`):

* _Low Likelihood (Not Expected)_: An event that is not expected to occur, but is still possible. Example: someone obtaining 51% mining power in Bitcoin.
* _Medium Likelihood (Could Happen)_: An event that could occur at some point, but is not likely to happen. Example: insufficient scalability of the system, likelihood depends a lot on design and implementation efforts.
* _High Likelihood (Expected)_: An event that is expected to occur, and is likely to happen. Example: software vulnerabilities, above certain complexity is very likely that bugs will be found.
* _Unknown Likelihood_: the likelihood of this event occurring is unclear. Example: Quantum Computing developed to attack blockchain, uncertain probability.

All risks are segmented by impact (use **bold** for level choice): 

* _High Impact_: the risk poses a significant threat to the sensitive information of a large number of users and has the potential to cause severe damage to the reputation of the client or result in substantial financial losses for both the client and the users. Example: 51% attack, or software vulnerabilities found.
* _Medium Impact_: The risk poses a risk to the sensitive information of a particular group of users, could harm the client's reputation if exploited, or has a reasonable chance of causing moderate financial consequences. Example: insufficient scalability, has an impact  but can be improved.
* _Low Impact_: The likelihood of the risk occurring repeatedly is low and it poses a relatively minor threat, or the user has indicated that considering their business situation, the impact of the risk is low. Example: Misconfigured Security Settings.
* _Informational (No Impact)_:  Although the problem does not pose an immediate risk, it is pertinent to follow security best practices or implement defense-in-depth measures. Example: Social engineering attacks, people has to be informed but is not affecting the software system per se.
* _Unknown Impact_: The consequences of the risk are unclear. Example: New Attack Techniques.
