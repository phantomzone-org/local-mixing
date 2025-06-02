import json
import numpy as np
import matplotlib.pyplot as plt
import sys
import os
from scipy.interpolate import griddata

def plot_heatmap(data, save_path):
    plt.clf()
    points = np.array(data)
    x, y, values = points[:, 0], points[:, 1], points[:, 2]

    mean = np.mean(values)
    std = np.std(values)
    if std == 0:
        std = 1

    z_scores = (values - mean) / std

    x_unique = np.unique(x)
    y_unique = np.unique(y)
    x_indices = {val: idx for idx, val in enumerate(x_unique)}
    y_indices = {val: idx for idx, val in enumerate(y_unique)}

    heatmap = np.full((len(y_unique), len(x_unique)), np.nan)
    for xi, yi, z in zip(x, y, z_scores):
        heatmap[y_indices[yi], x_indices[xi]] = z

    plt.imshow(
        heatmap,
        cmap='viridis',
        aspect='auto',
        origin='lower',
        extent=[x_unique[0], x_unique[-1], y_unique[0], y_unique[-1]],
    )
    plt.colorbar(label='Standard deviations from mean')
    plt.xlabel('x')
    plt.ylabel('y')

    os.makedirs(os.path.dirname(os.path.abspath(save_path)), exist_ok=True)
    try:
        plt.savefig(save_path, dpi=300)
    finally:
        plt.close()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python heatmap.py <json_path> <save_path>")
        sys.exit(1)
        
    json_path = sys.argv[1]
    save_path = sys.argv[2]
    
    try:
        with open(json_path, 'r') as f:
            data = json.load(f)
        plot_heatmap(data['results'], save_path)
    except FileNotFoundError:
        print(f"Error: Could not find file {json_path}")
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Error: Invalid JSON file {json_path}")
        sys.exit(1)
    except KeyError:
        print("Error: JSON file does not contain 'results' key")
        sys.exit(1)
