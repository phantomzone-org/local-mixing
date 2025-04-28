import json
import argparse
import os
import numpy as np
import matplotlib.pyplot as plt
from scipy.interpolate import griddata

def create_heatmap(data, input_str, output_dir):
    """Creates a heatmap for a single input's overlap data"""
    # Extract the data points
    tau1 = np.array([x[0] for x in data])
    tau2 = np.array([x[1] for x in data])
    overlap = np.array([x[2] for x in data])  # Don't square the overlap values

    # Create a very high resolution grid
    grid_x, grid_y = np.mgrid[0:1:1000j, 0:1:1000j]
    
    # Interpolate with high precision
    grid_z = griddata(
        (tau1, tau2), 
        overlap,  # Use raw overlap values
        (grid_x, grid_y), 
        method='cubic'
    )

    plt.figure(figsize=(16, 16))
    
    # Create tri-color map: red (-1) -> green (0) -> blue (1)
    colors = [(0.8, 0, 0), (0, 0.8, 0), (0, 0, 0.8)]  # red -> green -> blue
    cmap = plt.matplotlib.colors.LinearSegmentedColormap.from_list("custom", colors, N=256)
    
    # Plot with maximum resolution
    plt.imshow(grid_z.T, extent=[0, 1, 0, 1], origin='lower', 
              aspect='equal', cmap=cmap, vmin=-1, vmax=1,  # Set range to [-1, 1]
              interpolation='nearest')
    
    plt.colorbar(label='Overlap')  # Remove ^2 from label
    plt.title(f'Overlap Heatmap for input: {input_str}')
    plt.xlabel('tau1')
    plt.ylabel('tau2')
    
    # Save with very high DPI
    output_path = os.path.join(output_dir, f'heatmap_{input_str}.png')
    plt.savefig(output_path, dpi=300, bbox_inches='tight', pad_inches=0.1)
    plt.close()

def main():
    parser = argparse.ArgumentParser(description="Generate overlap heatmaps from distinguisher output.")
    parser.add_argument("input_file", help="Path to the input JSON file (save.json)")
    parser.add_argument("output_dir", help="Directory to save the heatmap plots")
    args = parser.parse_args()

    # Ensure output directory exists
    os.makedirs(args.output_dir, exist_ok=True)

    # Load the JSON data
    with open(args.input_file, 'r') as f:
        data = json.load(f)

    # Process each input string's data
    for input_str, overlap_data in data['results'].items():
        create_heatmap(overlap_data, input_str, args.output_dir)
        print(f"Generated heatmap for input: {input_str}")

if __name__ == "__main__":
    main()
