import json
import numpy as np
import matplotlib.pyplot as plt
import sys
import os

def plot_heatmap(data, save_path):
    plt.clf()
    
    points = np.array(data)
    x, y, values = points[:, 0], points[:, 1], points[:, 2]
    
    x_size = int(np.max(x) + 1)
    y_size = int(np.max(y) + 1)
    
    matrix = np.zeros((y_size, x_size))
    for x_pos, y_pos, val in points:
        matrix[int(y_pos), int(x_pos)] = val
    
    plt.imshow(matrix, cmap='viridis', aspect='auto', interpolation='nearest')
    plt.colorbar()
    
    os.makedirs(os.path.dirname(os.path.abspath(save_path)), exist_ok=True)
    
    try:
        plt.savefig(save_path)
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
