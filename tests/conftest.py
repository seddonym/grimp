import os
import sys

dirname = os.path.dirname(__file__)

# Add test assets to the Python path.

assets_path = os.path.abspath(os.path.join(dirname, "assets"))
sys.path.append(assets_path)

multiple_roots_path = os.path.abspath(os.path.join(dirname, "assets", "multipleroots"))
sys.path.append(multiple_roots_path)
