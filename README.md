# Nifty Generator

Nifty Generator (ng) is a command line tool for generating NFT images/video/metadata based on json configuration file. It works by randomly selecting attribute options and layering them to generate images. Video files can optionally be created if audio files are configured.

NOTE: this is still pre-release and as such, things such as config file structure are still subject to change. 

Basic usage:
    
    ng ../path/to/source/directory
    
The tool will expect to find a `config.json` in this source directory, which configures how the various media elements are to be combined along with corresponding metadata attribute values. A sample configuration file can be found at `config.template.json`. The resulting output will be generated within the `output` subdirectory.

A full listing of all available options can be found using:

    ng --help
