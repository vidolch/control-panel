#!/bin/sh

# Check if the user provided a sleep time argument
if [ $# -ne 2 ]; then
    echo "Usage: $0 <sleep_time_ms> <graceful_termination_ms>"
    exit 1
fi

SLEEP_TIMER=$1
TERMINATION_TIMER=$2

# Function to generate random data
generate_random_data() {
    echo "Random data: $RANDOM"
}


# Function to handle termination
terminate_script() {
  echo "Termination signal received. Exiting in $TERMINATION_TIMER seconds..."
    sleep "$TERMINATION_TIMER"
    exit 0
}

# Trap SIGINT (CTRL + C) and call terminate_script
trap terminate_script SIGINT

# Infinite loop to echo random data
while true; do
    # Output to stdout
    generate_random_data

    # Output to stderr
    # generate_random_data >&2

    # Sleep for 50 milliseconds
    sleep "$SLEEP_TIMER"
done

