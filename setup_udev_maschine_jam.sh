#!/bin/bash

# Setup udev rules for Native Instruments Maschine Jam
# This script creates udev rules to allow non-root access to the Maschine Jam

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        print_error "This script should not be run as root. Please run as a regular user."
        exit 1
    fi
}

# Function to get current user
get_current_user() {
    CURRENT_USER=$(whoami)
    print_status "Current user: $CURRENT_USER"
}

# Function to check if device is connected
check_device_connected() {
    print_status "Checking if Maschine Jam is connected..."
    if lsusb | grep -q "17cc:1500"; then
        print_success "Maschine Jam detected"
        return 0
    else
        print_warning "Maschine Jam not detected. Please connect the device and run this script again."
        return 1
    fi
}

# Function to create udev rule
create_udev_rule() {
    local rule_file="/etc/udev/rules.d/99-maschine-jam.rules"
    local rule_content="# Native Instruments Maschine Jam
SUBSYSTEM==\"usb\", ATTRS{idVendor}==\"17cc\", ATTRS{idProduct}==\"1500\", MODE=\"0666\", OWNER=\"$CURRENT_USER\", GROUP=\"$CURRENT_USER\""

    print_status "Creating udev rule file: $rule_file"
    
    # Check if rule file already exists
    if [[ -f "$rule_file" ]]; then
        print_warning "Udev rule file already exists. Backing up to ${rule_file}.backup"
        sudo cp "$rule_file" "${rule_file}.backup"
    fi
    
    # Create the rule file
    echo "$rule_content" | sudo tee "$rule_file" > /dev/null
    
    if [[ $? -eq 0 ]]; then
        print_success "Udev rule file created successfully"
    else
        print_error "Failed to create udev rule file"
        exit 1
    fi
}

# Function to reload udev rules
reload_udev_rules() {
    print_status "Reloading udev rules..."
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    
    if [[ $? -eq 0 ]]; then
        print_success "Udev rules reloaded successfully"
    else
        print_error "Failed to reload udev rules"
        exit 1
    fi
}

# Function to add user to plugdev group
add_to_plugdev_group() {
    print_status "Adding user to plugdev group..."
    sudo usermod -a -G plugdev "$CURRENT_USER"
    
    if [[ $? -eq 0 ]]; then
        print_success "User added to plugdev group"
        print_warning "You may need to log out and log back in for group changes to take effect"
    else
        print_error "Failed to add user to plugdev group"
        exit 1
    fi
}

# Function to verify setup
verify_setup() {
    print_status "Verifying setup..."
    
    # Check if rule file exists
    if [[ -f "/etc/udev/rules.d/99-maschine-jam.rules" ]]; then
        print_success "Udev rule file exists"
    else
        print_error "Udev rule file not found"
        return 1
    fi
    
    # Check if user is in plugdev group
    if groups "$CURRENT_USER" | grep -q plugdev; then
        print_success "User is in plugdev group"
    else
        print_warning "User is not in plugdev group (this is normal if you haven't logged out/in yet)"
    fi
    
    return 0
}

# Function to show next steps
show_next_steps() {
    echo
    print_status "Setup complete! Next steps:"
    echo "1. Unplug and replug your Maschine Jam"
    echo "2. Test the setup by running:"
    echo "   cargo run --bin simple-maschine-test"
    echo "3. If you get permission errors, log out and log back in"
    echo "4. For the main application, run:"
    echo "   cargo run --bin foreigninstruments-event-driven-rusb"
    echo
    print_warning "Note: You may need to log out and log back in for group changes to take effect"
}

# Function to show device info
show_device_info() {
    print_status "Device information:"
    echo "Vendor ID: 17cc"
    echo "Product ID: 1500"
    echo "Device: Native Instruments Maschine Jam"
    echo
}

# Main execution
main() {
    echo "=========================================="
    echo "  Maschine Jam Udev Rule Setup Script"
    echo "=========================================="
    echo
    
    # Check if not running as root
    check_root
    
    # Get current user
    get_current_user
    
    # Show device info
    show_device_info
    
    # Check if device is connected
    if ! check_device_connected; then
        exit 1
    fi
    
    # Create udev rule
    create_udev_rule
    
    # Add user to plugdev group
    add_to_plugdev_group
    
    # Reload udev rules
    reload_udev_rules
    
    # Verify setup
    verify_setup
    
    # Show next steps
    show_next_steps
    
    print_success "Setup completed successfully!"
}

# Run main function
main "$@" 