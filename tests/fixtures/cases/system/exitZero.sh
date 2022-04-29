#!/usr/bin/env bash

echo "stdout"
echo "stderr" >&2

exit 0

echo "This should not appear!"
