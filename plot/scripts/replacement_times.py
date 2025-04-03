import sys
import os
import re
import matplotlib.pyplot as plt
import numpy as np

def parse_log_file(file_path):
    """
    Plots replacement_times reading replacement_times.json

    python replacement_times.py <input.logs> <output_dir>
    """
    data = {
        "inflationary": [],
        "kneading": []
    }

    # Define the phrases to ignore
    ignore_phrases = [
        "Search has failed 100 times in a row",
        "Total number of iterations:",
        "Number of failed attempts:"
    ]

    try:
        with open(file_path, 'r') as file:
            for line in file:
                # Skip lines containing any of the ignore phrases
                if any(phrase in line for phrase in ignore_phrases):
                    continue

                # Extract n_circuits_sampled for inflationary and kneading steps
                if "inflationary" in line.lower():
                    match = re.search(r"n_circuits_sampled\s*=\s*(\d+)", line)
                    if match:
                        data["inflationary"].append(int(match.group(1)))
                elif "kneading" in line.lower():
                    match = re.search(r"n_circuits_sampled\s*=\s*(\d+)", line)
                    if match:
                        data["kneading"].append(int(match.group(1)))
    except FileNotFoundError:
        print(f"Error: File not found at {file_path}")
    except Exception as e:
        print(f"An error occurred: {e}")

    return data


def count_outliers(data):
    """
    Counts the number of outliers in the data using the 1.5 x IQR rule.

    Args:
        data (list): List of numerical values.

    Returns:
        int: Number of outliers.
    """
    q1 = np.percentile(data, 25)
    q3 = np.percentile(data, 75)
    iqr = q3 - q1
    lower_bound = q1 - 1.5 * iqr
    upper_bound = q3 + 1.5 * iqr

    outliers = [x for x in data if x < lower_bound or x > upper_bound]
    return len(outliers)


def plot_data(data, output_dir):
    # Ensure the output directory exists
    os.makedirs(output_dir, exist_ok=True)

    # Limit kneading data to 100k steps
    kneading_data = data["kneading"][:100000]
    inflationary_data = data["inflationary"]

    # Plot 1: Percentiles and outliers for inflationary
    plt.figure(figsize=(10, 6))
    plt.boxplot(inflationary_data, labels=["Inflationary"], showfliers=True)
    plt.title("Percentiles and Outliers of n_circuits_sampled (Inflationary)")
    plt.ylabel("n_circuits_sampled")
    plt.savefig(os.path.join(output_dir, "percentiles_outliers_inflationary.png"))
    plt.close()

    # Plot 2: Percentiles and outliers for kneading
    plt.figure(figsize=(10, 6))
    plt.boxplot(kneading_data, labels=["Kneading"], showfliers=True)
    plt.title("Percentiles and Outliers of n_circuits_sampled (Kneading)")
    plt.ylabel("n_circuits_sampled")
    plt.savefig(os.path.join(output_dir, "percentiles_outliers_kneading.png"))
    plt.close()

    # Plot 3: n_circuits_sampled over steps for inflationary (dots)
    plt.figure(figsize=(10, 6))
    plt.scatter(range(len(inflationary_data)), inflationary_data, label="Inflationary", alpha=0.7, s=10)
    plt.title("n_circuits_sampled Over Steps (Inflationary)")
    plt.xlabel("Step")
    plt.ylabel("n_circuits_sampled")
    plt.legend()
    plt.savefig(os.path.join(output_dir, "n_circuits_sampled_over_steps_inflationary.png"))
    plt.close()

    # Plot 4: n_circuits_sampled over steps for kneading (dots)
    plt.figure(figsize=(10, 6))
    plt.scatter(range(len(kneading_data)), kneading_data, label="Kneading", alpha=0.7, s=10)
    plt.title("n_circuits_sampled Over Steps (Kneading)")
    plt.xlabel("Step")
    plt.ylabel("n_circuits_sampled")
    plt.legend()
    plt.savefig(os.path.join(output_dir, "n_circuits_sampled_over_steps_kneading.png"))
    plt.close()

    # Print the number of outliers for each stage
    inflationary_outliers = count_outliers(inflationary_data)
    kneading_outliers = count_outliers(kneading_data)
    print(f"Number of outliers in inflationary data: {inflationary_outliers}")
    print(f"Number of outliers in kneading data: {kneading_outliers}")


if __name__ == "__main__":
    # Check if the user provided the required arguments
    if len(sys.argv) != 3:
        print("Usage: python parse_log.py <logfile_path> <output_directory>")
        sys.exit(1)

    # Get the log file path and output directory from the command-line arguments
    log_file_path = sys.argv[1]
    output_directory = sys.argv[2]

    # Parse the log file
    parsed_data = parse_log_file(log_file_path)

    # Generate and save the plots, and print the number of outliers
    plot_data(parsed_data, output_directory)

    print(f"Plots saved to {output_directory}")