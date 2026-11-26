#!/usr/bin/env python3
"""1 Hz host monitor for the Pi timing runs. Usage: monitor.py CORE OUTFILE
Logs: epoch, loadavg1, freq_khz(core), temp_mC, volt_alarm, core jiffies (user nice sys idle iowait irq softirq steal),
and the summed utime+stime jiffies of every process whose comm is 'stage2c*' (the measured binary)."""
import os, sys, time
core, out = int(sys.argv[1]), sys.argv[2]
def rd(p, d=""):
    try:
        return open(p).read().strip()
    except OSError:
        return d
hw = {rd(f"/sys/class/hwmon/{h}/name"): f"/sys/class/hwmon/{h}" for h in os.listdir("/sys/class/hwmon")}
def own():
    tot = 0
    for pid in filter(str.isdigit, os.listdir("/proc")):
        try:
            s = open(f"/proc/{pid}/stat").read()
            comm = s[s.index("(") + 1:s.rindex(")")]
            if comm.startswith("stage2c"):
                f = s[s.rindex(")") + 2:].split()
                tot += int(f[11]) + int(f[12])
        except (OSError, ValueError):
            pass
    return tot
with open(out, "w", buffering=1) as o:
    o.write("epoch,load1,freq_khz,temp_mC,volt_alarm,user,nice,sys,idle,iowait,irq,softirq,steal,own_jiffies\n")
    while True:
        cpu = next((l.split()[1:9] for l in open("/proc/stat") if l.startswith(f"cpu{core} ")), ["0"] * 8)
        o.write(",".join(map(str, [
            f"{time.time():.1f}", rd("/proc/loadavg").split()[0],
            rd(f"/sys/devices/system/cpu/cpu{core}/cpufreq/scaling_cur_freq"),
            rd(hw.get("cpu_thermal", "") + "/temp1_input"),
            rd(hw.get("rpi_volt", "") + "/in0_lcrit_alarm", "na"), *cpu, own()])) + "\n")
        time.sleep(1)
