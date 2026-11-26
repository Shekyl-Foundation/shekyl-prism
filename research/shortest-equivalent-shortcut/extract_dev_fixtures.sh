#!/bin/sh
# Copies the dev stage2c vectors/ideals (origin/dev 470d537) to /tmp/dev_fx, where run_real.py and dev_tally.py read them.
set -e
for s in p248 p324 p376 p500 p664; do for d in vectors ideals; do
  mkdir -p /tmp/dev_fx/$d
  git show 470d537:shekyl-prism/examples/stage2c/$d/$s.txt > /tmp/dev_fx/$d/$s.txt
done; done
