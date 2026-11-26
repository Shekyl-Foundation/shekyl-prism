#!/bin/bash
# Hardware and load state. Usage: snapshot.sh CORE
C=${1:-3}
echo "== $(date -Is) core=$C"
uptime; who; cat /proc/loadavg
echo "governors: $(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor | sort | uniq -c | tr '\n' ' ')"
for i in 0 1 2 3; do echo "cpu$i freq=$(cat /sys/devices/system/cpu/cpu$i/cpufreq/scaling_cur_freq) min=$(cat /sys/devices/system/cpu/cpu$i/cpufreq/scaling_min_freq) max=$(cat /sys/devices/system/cpu/cpu$i/cpufreq/scaling_max_freq)"; done
for h in /sys/class/hwmon/hwmon*; do n=$(cat $h/name); echo "hwmon $n: $(for f in $h/temp1_input $h/in0_lcrit_alarm $h/in0_input; do [ -r $f ] && echo -n "$(basename $f)=$(cat $f) "; done)"; done
echo "vcgencmd: $( (vcgencmd get_throttled) 2>&1 | head -1)"
echo "cmdline: $(cat /proc/cmdline)"
echo "aslr: $(cat /proc/sys/kernel/randomize_va_space) thp: $(cat /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null)"
echo "tasks on core $C (psr pcpu comm):"; ps -eLo psr,pcpu,comm --sort=-pcpu | awk -v c=$C '$1==c' | head -6
echo "top cpu:"; ps -eo pid,pcpu,comm --sort=-pcpu | head -6
echo "shekyl daemons: $(pgrep -c shekyld) shekyld, $(pgrep -c -f shekyl-wallet) wallet"
grep -E "^(Mem|Swap)" /proc/meminfo | head -3
