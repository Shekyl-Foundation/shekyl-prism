## Headline: largest intermediate, proven vs observed (bits), against the prism-int width

| Set | Variant (rep) | runs | proven max | observed max | width | spare (proven) | spare (observed) | lemma viol. | oracle fail | saturated q |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| p248 | O0:keygen_D | 500 | 1029 | 1027 | 1664 | 635 | 637 | 0 | 0/500 | 0 |
| p248 | O0:sign_generic | 3603 | 1303 | 1301 | 1664 | 361 | 363 | 0 | 0/1353 | 600 |
| p248 | model:Iq_only | 1000 | 985 | 983 | 1664 | 679 | 681 | 0 | 0/500 | 0 |
| p248 | model:keygen_D | 1000 | 1027 | 1025 | 1664 | 637 | 639 | 0 | 0/500 | 0 |
| p248 | model:keygen_prereduce_Isec | 1000 | 319 | 317 | 1664 | 1345 | 1347 | 0 | 0/1000 | 0 |
| p248 | model:sign_generic | 9609 | 1301 | 1299 | 1664 | 363 | 365 | 0 | 0/2609 | 603801 |
| p248 | model:sign_prereduced_guarded | 3222 | 1268 | 1267 | 1664 | 396 | 397 | 0 | 0/3222 | 0 |
| p248 | model:sign_prereduced_unguarded_generic | 2478 | 1300 | 1298 | 1664 | 364 | 366 | 0 | 0/2478 | 0 |
| p324 | O0:keygen_D | 500 | 1541 | 1539 | 2112 | 571 | 573 | 0 | 0/500 | 0 |
| p324 | O0:sign_generic | 3603 | 1681 | 1679 | 2112 | 431 | 433 | 0 | 0/1353 | 772 |
| p324 | model:Iq_only | 1000 | 1289 | 1287 | 2112 | 823 | 825 | 0 | 0/500 | 0 |
| p324 | model:keygen_D | 1000 | 1539 | 1537 | 2112 | 573 | 575 | 0 | 0/500 | 0 |
| p324 | model:keygen_prereduce_Isec | 1000 | 393 | 391 | 2112 | 1719 | 1721 | 0 | 0/1000 | 0 |
| p324 | model:sign_generic | 9609 | 1679 | 1677 | 2112 | 433 | 435 | 0 | 0/2609 | 776317 |
| p324 | model:sign_prereduced_guarded | 3183 | 1647 | 1645 | 2112 | 465 | 467 | 0 | 0/3183 | 0 |
| p324 | model:sign_prereduced_unguarded_generic | 2517 | 1678 | 1676 | 2112 | 434 | 436 | 0 | 0/2517 | 0 |
| p376 | O0:keygen_D | 500 | 1541 | 1539 | 2432 | 891 | 893 | 0 | 0/500 | 0 |
| p376 | O0:sign_generic | 3603 | 1947 | 1945 | 2432 | 485 | 487 | 0 | 0/1353 | 890 |
| p376 | model:Iq_only | 1000 | 1497 | 1495 | 2432 | 935 | 937 | 0 | 0/500 | 0 |
| p376 | model:keygen_D | 1000 | 1539 | 1537 | 2432 | 893 | 895 | 0 | 0/500 | 0 |
| p376 | model:keygen_prereduce_Isec | 1000 | 451 | 449 | 2432 | 1981 | 1983 | 0 | 0/1000 | 0 |
| p376 | model:sign_generic | 9609 | 1945 | 1943 | 2432 | 487 | 489 | 0 | 0/2609 | 894671 |
| p376 | model:sign_prereduced_guarded | 3150 | 1912 | 1910 | 2432 | 520 | 522 | 0 | 0/3150 | 0 |
| p376 | model:sign_prereduced_unguarded_generic | 2550 | 1944 | 1942 | 2432 | 488 | 490 | 0 | 0/2550 | 0 |
| p500 | O0:keygen_D | 500 | 2053 | 2051 | 3136 | 1083 | 1085 | 0 | 0/500 | 0 |
| p500 | O0:sign_generic | 3603 | 2565 | 2563 | 3136 | 571 | 573 | 0 | 0/1353 | 1172 |
| p500 | model:Iq_only | 1000 | 1993 | 1991 | 3136 | 1143 | 1145 | 0 | 0/500 | 0 |
| p500 | model:keygen_D | 1000 | 2051 | 2049 | 3136 | 1085 | 1087 | 0 | 0/500 | 0 |
| p500 | model:keygen_prereduce_Isec | 1000 | 573 | 571 | 3136 | 2563 | 2565 | 0 | 0/1000 | 0 |
| p500 | model:sign_generic | 9609 | 2563 | 2561 | 3136 | 573 | 575 | 0 | 0/2609 | 1177517 |
| p500 | model:sign_prereduced_guarded | 3240 | 2530 | 2529 | 3136 | 606 | 607 | 0 | 0/3240 | 0 |
| p500 | model:sign_prereduced_unguarded_generic | 2460 | 2562 | 2560 | 3136 | 574 | 576 | 0 | 0/2460 | 0 |
| p664 | O0:keygen_D | 500 | 3077 | 3075 | 4160 | 1083 | 1085 | 0 | 0/500 | 0 |
| p664 | O0:sign_generic | 3603 | 3385 | 3383 | 4160 | 775 | 777 | 0 | 0/1353 | 1544 |
| p664 | model:Iq_only | 1000 | 2649 | 2647 | 4160 | 1511 | 1513 | 0 | 0/500 | 0 |
| p664 | model:keygen_D | 1000 | 3075 | 3073 | 4160 | 1085 | 1087 | 0 | 0/500 | 0 |
| p664 | model:keygen_prereduce_Isec | 1000 | 737 | 735 | 4160 | 3423 | 3425 | 0 | 0/1000 | 0 |
| p664 | model:sign_generic | 9609 | 3383 | 3381 | 4160 | 777 | 779 | 0 | 0/2609 | 1550633 |
| p664 | model:sign_prereduced_guarded | 3099 | 3350 | 3349 | 4160 | 810 | 811 | 0 | 0/3099 | 0 |
| p664 | model:sign_prereduced_unguarded_generic | 2601 | 3382 | 3380 | 4160 | 778 | 780 | 0 | 0/2601 | 0 |

## Per intermediate class, signing generic path in O0 numerators (proven / observed bits)

| Intermediate | p248 | p324 | p376 | p500 | p664 |
|---|---:|---:|---:|---:|---:|
| starting norms N4 (in) | 1301 / 1301 | 1679 / 1679 | 1945 / 1945 | 2563 / 2563 | 3383 / 3383 |
| coordinates, step and conversion | 651 / 650 | 840 / 839 | 973 / 972 | 1282 / 1281 | 1692 / 1691 |
| quotient x coordinate (m u) | 652 / 651 | 841 / 840 | 974 / 973 | 1283 / 1282 | 1693 / 1692 |
| update temporaries a = mu u | 652 / 651 | 841 / 840 | 974 / 973 | 1283 / 1282 | 1693 / 1692 |
| Hermitian products / partial sums / h | 1302 / 1300 | 1680 / 1678 | 1946 / 1944 | 2564 / 2562 | 3384 / 3382 |
| j-part sums before x p | 1051 / 802 | 1354 / 1030 | 1563 / 1183 | 2059 / 1555 | 2715 / 2049 |
| norms N4 and partial sums | 1302 / 1300 | 1680 / 1678 | 1946 / 1944 | 2564 / 2562 | 3384 / 3382 |
| 64-bit quotient |m| | 63 / 61 | 63 / 61 | 63 / 61 | 63 / 61 | 63 / 61 |
| 64-bit quotient internals (hs, num) | 126 / 126 | 126 / 126 | 126 / 126 | 126 / 126 | 126 / 126 |
| cleanup exact quotient |m| | 651 / 401 | 840 / 515 | 973 / 591 | 1282 / 778 | 1692 / 1024 |
| cleanup rounding numerator 2h + nu | 1303 / 777 | 1681 / 1004 | 1947 / 1165 | 2565 / 1535 | 3385 / 2027 |
| cleanup rounding denominator 2 nu | 1303 / 777 | 1681 / 1004 | 1947 / 1165 | 2565 / 1535 | 3385 / 2027 |
| 4-D Gram entries (N4 scale) | 1302 / 1300 | 1680 / 1678 | 1946 / 1944 | 2564 / 2562 | 3384 / 3382 |
| 4-D Gram entries (reference 2x scale) | 1303 / 1301 | 1681 / 1679 | 1947 / 1945 | 2565 / 2563 | 3385 / 3383 |

## Keygen guard nrd(v_s) <= c sqrt(p) N on sampled keys: rejections

| Set | keys | c=2 | c=4 | c=16 | c=256 | c=2^16 | max nrd(v_s)/(sqrt(p) N) |
|---|---:|---:|---:|---:|---:|---:|---:|
| p248 | 10000 | 2261 | 494 | 32 | 0 | 0 | 77.73 |
| p324 | 10000 | 2228 | 512 | 39 | 0 | 0 | 92.41 |
| p376 | 10000 | 2141 | 474 | 30 | 0 | 0 | 72.40 |
| p500 | 10000 | 2232 | 519 | 34 | 0 | 0 | 111.87 |
| p664 | 10000 | 2272 | 548 | 32 | 0 | 0 | 153.71 |

## Q4 slack: largest B for which the reduction still fits the width

| Set | B (Q4) | max B, generic shape | max B, any shape |
|---|---:|---:|---:|
| p248 | 158 | 338 | 213 |
| p324 | 195 | 410 | 247 |
| p376 | 224 | 466 | 275 |
| p500 | 285 | 570 | 318 |
| p664 | 367 | 754 | 420 |
