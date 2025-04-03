import json
import argparse
import os
import matplotlib.pyplot as plt

def plot_hamming_weights(data, output_folder):
    """
    Plots the data produced from 'cargo run distinguisher'. 
    Plot is x = gate index, y = hamming weight. Red is "circuit_one", blue is "circuit_two".

    python hamming_weight.py <input_file> <output_dir>
    """
    results = data["results"]

    # Ensure the output folder exists
    os.makedirs(output_folder, exist_ok=True)

    for input_binary, (hamming_weights_one, hamming_weights_two) in results.items():
        plt.figure(figsize=(10, 6))  # Adjust figure size for individual plots

        plt.plot(
            range(len(hamming_weights_one)),
            hamming_weights_one,
            color="red",
            marker="o",
            linestyle="-",
            alpha=0.5,
            label="circuit 1",
        )
        plt.plot(
            range(len(hamming_weights_two)),
            hamming_weights_two,
            color="blue",
            marker="o",
            linestyle="-",
            alpha=0.5,
            label="circuit 2",
        )

        plt.xlabel("Index", fontsize=14)
        plt.ylabel("Hamming Weight", fontsize=14)
        plt.legend(loc="upper right", fontsize=10)
        plt.grid(True, which="both", axis="x", linestyle="--", linewidth=0.5)

        output_path = os.path.join(output_folder, f"{input_binary}.png")
        plt.savefig(output_path)
        plt.close()  # Close the figure to free memory
        print(f"Plot saved to {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Plot Hamming weights from JSON data.")
    parser.add_argument("input_file", help="Path to the input JSON file (e.g., d.json).")
    parser.add_argument("output_folder", help="Path to the folder to save the output plot images.")
    args = parser.parse_args()

    with open(args.input_file, "r") as f:
        data = json.load(f)

    plot_hamming_weights(data, args.output_folder)


if __name__ == "__main__":
    main()
