awk -F, '{ sum += $2; count += 1 } END { if (count > 0) printf "%.10f\n", sum / count }'
