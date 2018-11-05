import sys
import os


# Add test assets to the Python path.
dirname = os.path.dirname(__file__)
path = os.path.abspath(os.path.join(dirname, 'assets'))
sys.path.append(path)
