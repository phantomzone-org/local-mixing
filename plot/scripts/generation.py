import json
import argparse
import os
import matplotlib.pyplot as plt
import numpy as np

def plot_data(data, output_path, downsample_factor=1, alpha=0.5):
    """
    Plots generation data with x-axis as gate index and y-axis as generation value.
    Saves the plot to the specified output path.

    Args:
        data (list): The generation data to plot.
        output_path (str): Path to save the plot image.
        downsample_factor (int): Factor by which to downsample the data for clarity.
        alpha (float): Transparency level for the plot points.
    """
    # Downsample data if needed
    if downsample_factor > 1:
        data = data[::downsample_factor]

    # X-axis: index (gate)
    x = range(len(data))
    # Y-axis: value (generation)
    y = data

    plt.figure(figsize=(10, 6))
    plt.scatter(x, y, color='b', alpha=alpha, label='Generation')  # Use scatter plot
    plt.title('Generation vs Gate')
    plt.xlabel('Gate')
    plt.ylabel('Generation')
    plt.legend()
    plt.grid(True)
    plt.savefig(output_path)
    plt.close()  # Close the figure to free memory
    print(f"Plot saved to {output_path}")

def main():
    parser = argparse.ArgumentParser(description="Plot generation data from JSON.")
    parser.add_argument("input_file", help="Path to the input JSON file (e.g., generation.json).")
    parser.add_argument("output_folder", help="Path to the folder to save the output plot image.")
    parser.add_argument("--downsample", type=int, default=1, help="Factor to downsample the data.")
    parser.add_argument("--alpha", type=float, default=0.5, help="Transparency level for the plot.")
    args = parser.parse_args()

    # Ensure the output folder exists
    os.makedirs(args.output_folder, exist_ok=True)

    # Load data from JSON file
    with open(args.input_file, "r") as f:
        data = json.load(f)

    # Handle both list and dictionary formats
    if isinstance(data, dict):
        generation_data = data.get("generation", [])
    elif isinstance(data, list):
        generation_data = data
    else:
        print("Unsupported JSON format. Expected a list or a dictionary.")
        return

    if not generation_data:
        print("No generation data found in the input file.")
        return

    # Define output path
    output_path = os.path.join(args.output_folder, "generation_plot.png")

    # Plot and save the data
    plot_data(generation_data, output_path, downsample_factor=args.downsample, alpha=args.alpha)

if __name__ == "__main__":
    main()