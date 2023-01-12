#!/usr/bin/env python
# -*- encoding: utf-8 -*-
import io
from glob import glob
from os.path import basename
from os.path import dirname
from os.path import join
from os.path import splitext

from setuptools import find_packages
from setuptools import setup


def read(*names, **kwargs):
    with io.open(
        join(dirname(__file__), *names),
        encoding=kwargs.get('encoding', 'utf8')
    ) as fh:
        return fh.read()


setup(
    name='grimp',
    version='2.2',
    license='BSD 2-Clause License',
    description="Builds a queryable graph of the imports within one or more Python packages.",
    long_description=read('README.rst'),
    long_description_content_type='text/x-rst',
    author='David Seddon',
    author_email='david@seddonym.me',
    project_urls={
        'Documentation': 'https://grimp.readthedocs.io/',
        'Source code': 'https://github.com/seddonym/grimp/',
    },
    packages=find_packages('src'),
    package_data={'grimp': ['py.typed']},
    package_dir={'': 'src'},
    py_modules=[splitext(basename(path))[0] for path in glob('src/*.py')],
    include_package_data=True,
    zip_safe=False,
    classifiers=[
        # complete classifier list: http://pypi.python.org/pypi?%3Aaction=list_classifiers
        'Development Status :: 5 - Production/Stable',
        'Intended Audience :: Developers',
        'License :: OSI Approved :: BSD License',
        'Operating System :: Unix',
        'Operating System :: POSIX',
        'Operating System :: Microsoft :: Windows',
        'Programming Language :: Python',
        'Programming Language :: Python :: 3.7',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: 3.10',
        'Programming Language :: Python :: 3.11',
        'Programming Language :: Python :: Implementation :: CPython',
        'Topic :: Utilities',
    ],
    python_requires=">=3.7",
    install_requires=[
        "typing-extensions>=3.10.0.0",
    ],
)
