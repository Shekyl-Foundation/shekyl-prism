#!/usr/bin/env Rscript
# Batch driver for the pinned RTLF reference implementation.
#
# Sources the authors' `test`, `bootstrap1` and `autotest` functions verbatim
# from rtlf.R (tls-attacker/RTLF @ 2497d660e4b2, USENIX Security 2024) and calls
# them exactly as rtlf.R's process_file does: n = min(class sizes), B = 10000,
# alpha per decile = alpha / 9. Only the CLI, the CSV reader and the report
# printing are bypassed; inputs here are already clean two-class vectors.
# Each input gets its own seed (derived from its name) so reruns reproduce.
#
# Usage: rtlf_batch.R RTLF_R LISTFILE OUT.tsv ALPHA CORES
suppressPackageStartupMessages({ library(dplyr); library(parallel) })
a <- commandArgs(trailingOnly = TRUE)
src <- readLines(a[1])
lo <- grep("^# Test function", src); hi <- grep("^# Output Handling", src)
eval(parse(text = src[lo:(hi - 2)]))
files <- readLines(a[2]); alpha <- as.numeric(a[4]); cores <- as.integer(a[5])
one <- function(f) {
  seed <- strtoi(substr(digest_hex(f), 1, 7), 16L)
  set.seed(seed)
  d <- read.csv(f, header = TRUE, colClasses = c("character", "numeric"))
  n <- min(sum(d$V1 == "1"), sum(d$V1 == "2"))
  o <- autotest(d, n, 10000, alpha / 9)
  paste(f, n, max(o[[1]]), paste(which(o[[1]] > 0) * 10, collapse = ";"),
        paste(o[[2]], collapse = ";"), paste(o[[3]], collapse = ";"), sep = "\t")
}
digest_hex <- function(s) {  # tiny FNV-1a, enough for a per-file seed
  h <- 2166136261
  for (ch in utf8ToInt(s)) { h <- bitwXor(as.integer(h %% 2^31), ch); h <- (h * 16777619) %% 2^32 }
  sprintf("%08x", as.integer(h %% 2^31))
}
res <- mclapply(files, one, mc.cores = cores, mc.preschedule = FALSE)
writeLines(c("file\tn\tdetect\tsig_deciles\tdiff\tthreshold", unlist(res)), a[3])
